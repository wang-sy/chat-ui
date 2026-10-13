"""Capture real artifacts for review; structural signals are NOT quality passes."""
import json
from collections import Counter
from html.parser import HTMLParser
from pathlib import Path

from client import OUTPUT, capture, digest, save


class Structure(HTMLParser):
    def __init__(self, html):
        super().__init__()
        self.tags = Counter()
        self.embeds = []
        self.labels = []
        self.in_label = False
        self.feed(html)

    def handle_starttag(self, tag, attrs):
        self.tags[tag] += 1
        if tag == "chat-view":
            self.embeds.append(dict(attrs).get("view-id"))
        if tag == "text":
            self.in_label = True

    def handle_endtag(self, tag):
        if tag == "text":
            self.in_label = False

    def handle_data(self, text):
        if self.in_label and text.strip():
            self.labels.append(text.strip())

    def facts(self):
        return {"elements": {k: self.tags[k] for k in (
            "svg", "path", "line", "text", "form", "input", "select", "button")},
            "literalSvgLabels": self.labels, "embeds": self.embeds}


def artifact(page):
    html = page.get("html", "")
    return {"title": page.get("title"), "summary": page.get("summary"),
            "sha256": digest(page), "bytes": len(html.encode()),
            "structure": Structure(html).facts()}


def inspect(case):
    cwd = OUTPUT / (case["id"] + "-" + case["title"])
    if not cwd.exists():
        return {"id": case["id"], "execution": "not-run"}
    snapshot = capture(cwd, "visual-observed")
    space, chats = snapshot["space"], snapshot["chats"]
    views = space["views"]
    main = artifact(space["mainPage"])
    refs = set(main["structure"]["embeds"])
    covered = all(v["id"] in refs or v.get("key") in refs for v in views)
    prompts = [json.loads(line) for line in
               (cwd / "evidence" / "requests.jsonl").read_text().splitlines()]
    return {"id": case["id"], "title": case["title"], "at": snapshot["at"],
            "requestCount": len(prompts), "mainThreadId": space["mainThreadId"],
            "integration": space.get("integration"), "allAuthoredEmbeds": covered,
            "main": main,
            "views": [{"id": v["id"], "key": v.get("key"), "kind": v["kind"],
                       "threadId": v["threadId"], "status": v["status"],
                       "brief": v["brief"], **artifact(v)} for v in views],
            "execution": [{k: c.get(k) for k in (
                "id", "role", "activeTurn", "activity", "error")} for c in chats],
            "browserAcceptance": "see separately recorded browser evidence"}


if __name__ == "__main__":
    cases = json.loads(Path(__file__).with_name("visual-cases.json").read_text())
    report = [inspect(case) for case in cases]
    save(OUTPUT / "visual-observed.json", report)
    for row in report:
        print(json.dumps({k: row.get(k) for k in (
            "id", "title", "requestCount", "allAuthoredEmbeds", "execution")},
            ensure_ascii=False))
