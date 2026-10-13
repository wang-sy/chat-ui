"""Real local-service calls. No mock model, page HTML, or synthetic Codex events."""
import hashlib
import json
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "live-agent-cases"
BASE = "http://127.0.0.1:4317"


def now():
    return time.strftime("%Y-%m-%dT%H:%M:%S%z")


def api(path, body=None):
    health = json.load(urllib.request.urlopen(BASE + "/health", timeout=10))
    headers = {"X-Chat-UI-Token": health["token"]}
    data = None
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    try:
        return json.load(urllib.request.urlopen(
            urllib.request.Request(BASE + path, data=data, headers=headers), timeout=65))
    except urllib.error.HTTPError as error:
        raise RuntimeError(error.read().decode()) from error


def save(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")


def space(cwd):
    return api("/api/space?" + urllib.parse.urlencode({"cwd": str(cwd)}))


def state(cwd):
    return [chat for chat in api("/api/state")["chats"] if chat["cwd"] == str(cwd)]


def capture(cwd, label):
    snapshot = {"at": now(), "space": space(cwd), "chats": state(cwd)}
    save(cwd / "evidence" / (label + ".json"), snapshot)
    return snapshot


def digest(page):
    return hashlib.sha256(page.get("html", "").encode()).hexdigest()


def summary(snapshot):
    s = snapshot["space"]
    return {"at": snapshot["at"], "revision": s["revision"],
            "mainThreadId": s["mainThreadId"],
            "mainHtmlBytes": len(s["mainPage"]["html"].encode()),
            "views": [{k: v.get(k) for k in ("id", "title", "threadId", "status", "version")}
                      | {"htmlBytes": len(v["html"].encode()), "sha256": digest(v)}
                      for v in s["views"]],
            "chats": [{k: c.get(k) for k in
                       ("id", "role", "viewId", "activeTurn", "activity", "error")}
                      for c in snapshot["chats"]]}


def prepare(case):
    cwd = OUTPUT / (case["id"] + "-" + case["title"])
    cwd.mkdir(parents=True, exist_ok=True)
    (cwd / "AGENTS.md").write_text(
        "# Live Chat UI acceptance case\n\n"
        "The user explicitly authorized running this case inside this directory. "
        "Only modify this case directory. Do not edit the host Chat UI application, "
        "other case folders, existing user projects, credentials, or global configuration. "
        "Use the supplied Main/View structured output contract. Generated pages must "
        "be self-contained and interactive, with no external dependencies. "
        "Do not read evidence/ or .space/ for answers; runtime provides the current context. "
        "The host verifies browser interaction after delivery; do not open a browser, "
        "start a preview server or request browser-tool access for validation. "
        "This is a real run, so do not claim tests passed without executing them.\n")
    save(cwd / "case.json", case)
    api("/api/projects", {"path": str(cwd)})
    before = capture(cwd, "initial")
    # Existing app seeds three unrelated demo pages; remove them as explicit setup.
    # Preserve this evidence rather than mistaking them for agent-generated output.
    if not before["space"]["mainThreadId"]:
        for view in before["space"]["views"]:
            if view["threadId"] is None:
                api("/api/space", {"cwd": str(cwd), "action": "deleteView", "viewId": view["id"]})
    return cwd


def send(cwd, text, *, view=None, request_id=None, effort="high"):
    body = {"cwd": str(cwd), "model": "gpt-6.1-sol", "effort": effort,
            "speed": "standard", "permission": "ask", "text": text,
            "requestId": request_id or ("live-" + uuid.uuid4().hex)}
    if view:
        body["viewId"] = view
    response = api("/api/chats/send", body)
    record = {"at": now(), "request": body, "response": response}
    with (cwd / "evidence" / "requests.jsonl").open("a") as stream:
        stream.write(json.dumps(record, ensure_ascii=False) + "\n")
    return response


def wait(cwd, label="result", timeout=900):
    started = time.monotonic()
    previous = None
    quiet = 0
    while time.monotonic() - started < timeout:
        snapshot = {"at": now(), "space": space(cwd), "chats": state(cwd)}
        compact = summary(snapshot)
        signature = json.dumps({k: v for k, v in compact.items() if k != "at"}, sort_keys=True)
        if signature != previous:
            with (cwd / "evidence" / "timeline.jsonl").open("a") as stream:
                stream.write(json.dumps(compact, ensure_ascii=False) + "\n")
            print(json.dumps({"case": cwd.name, "elapsed": round(time.monotonic() - started),
                              "views": [(v["title"], v["status"]) for v in compact["views"]],
                              "activity": [(c["role"], c["activity"], c["error"]) for c in compact["chats"]]},
                             ensure_ascii=False), flush=True)
            previous = signature
        requests = [r for c in snapshot["chats"] for r in c.get("requests", [])]
        if requests:
            save(cwd / "evidence" / (label + "-pending.json"), snapshot)
            return {"pending": True, "requests": requests}
        busy = any(c.get("activeTurn") or (c.get("activity") and c["activity"] != "已停止")
                   for c in snapshot["chats"])
        busy |= any(v["status"] in ("planning", "working") for v in snapshot["space"]["views"])
        quiet = 0 if busy else quiet + 1
        if quiet >= 3:
            save(cwd / "evidence" / (label + ".json"), snapshot)
            return compact
        time.sleep(2)
    save(cwd / "evidence" / (label + "-timeout.json"), snapshot)
    return {"timeout": True, "last": compact}
