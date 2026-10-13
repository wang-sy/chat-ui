"""Record public transcript growth from the real service, scoped to one case."""
import json
import sys
import time
import urllib.request
from client import BASE, OUTPUT, now

cwd = OUTPUT / "历史与流式验收"
filename = sys.argv[1] if len(sys.argv) > 1 else "stream.jsonl"
deadline = time.monotonic() + 900
cursor = 0
session = 0
previous_token = None
while time.monotonic() < deadline:
    try:
        health = json.load(urllib.request.urlopen(BASE + "/health", timeout=10))
        if previous_token != health["token"]:
            previous_token = health["token"]
            session += 1
            cursor = 0
        request = urllib.request.Request(BASE + "/api/events?since=" + str(cursor),
                                         headers={"X-Chat-UI-Token": health["token"]})
        with urllib.request.urlopen(request, timeout=30) as response:
            for line in response:
                if time.monotonic() >= deadline:
                    break
                if not line.startswith(b"data: "):
                    continue
                event = json.loads(line[6:])
                if event.get("sequence", 0) <= cursor:
                    continue
                cursor = event.get("sequence", cursor)
                owner = event.get("params", {}).get("threadId")
                for chat in event.get("state", {}).get("chats", []):
                    if chat.get("cwd") != str(cwd) or chat["id"] != owner:
                        continue
                    entry = {"at": now(), "session": session, "sequence": cursor, "method": event["method"],
                             "threadId": owner, "role": chat.get("role"),
                             "activeTurn": chat.get("activeTurn"), "error": chat.get("error"),
                             "items": [{"id": i["id"], "kind": i["kind"], "status": i["status"],
                                        "chars": len(i["text"]), "tail": i["text"][-160:]}
                                       for i in chat.get("transcript", [])[-6:]]}
                    with (cwd / "evidence" / filename).open("a") as stream:
                        stream.write(json.dumps(entry, ensure_ascii=False) + "\n")
    except (OSError, ValueError) as error:
        print(type(error).__name__, flush=True)
        time.sleep(1)
