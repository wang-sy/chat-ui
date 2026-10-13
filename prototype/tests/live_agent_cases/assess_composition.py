"""Verify recorded real C02 composition runs; never fabricate model results."""
import json
from html.parser import HTMLParser
from client import OUTPUT, digest, now, save


class Elements(HTMLParser):
    def __init__(self, html):
        super().__init__()
        self.embeds = []
        self.svg_count = 0
        self.feed(html)

    def handle_starttag(self, tag, attrs):
        if tag == "chat-view":
            self.embeds.append(dict(attrs).get("view-id"))
        if tag == "svg":
            self.svg_count += 1


case = OUTPUT / "C02-推荐工作台" / "evidence"
before = json.loads((case / "visual-composition-before.json").read_text())
after = json.loads((case / "visual-composition.json").read_text())
interaction = json.loads((case / "embedded-update-before.json").read_text())
updated = json.loads((case / "embedded-update.json").read_text())
page = after["space"]
old_views = {v["id"]: v for v in before["space"]["views"]}
views = {v["id"]: v for v in page["views"]}
next_views = {v["id"]: v for v in updated["space"]["views"]}
embeds = Elements(page["mainPage"]["html"]).embeds
references = {v["key"]: v["id"] for v in views.values() if v.get("key")}
references.update({v: v for v in views})
events = [json.loads(line) for line in (case / "events.jsonl").read_text().splitlines()]
integrations = [e for e in events if e["at"] >= before["at"]
                and e["method"] == "chat-ui/main/integrationStarted"]
checks = {
    "复用两个原 View 及其 Thread": len(views) == 2 and views.keys() == old_views.keys()
        and all(v["threadId"] == old_views[k]["threadId"] for k, v in views.items()),
    "Main 内嵌两个真实子结果": len(embeds) == 2
        and {references.get(ref) for ref in embeds} == views.keys(),
    "两个子结果均含 SVG 且已就绪": all(v["status"] == "ready"
        and Elements(v["html"]).svg_count > 0 for v in views.values()),
    "有明确的持久化 View 关系": bool(page["connections"])
        and all(e["from"] in views and e["to"] in views and e.get("label")
                for e in page["connections"]),
    "全部子任务交付后仅一次 Main 整合": page["integration"]["status"] == "complete"
        and len({e["sequence"] for e in integrations}) == 1,
    "浏览器选择与导航没有发起新的 Agent 请求":
        sum(m["role"] == "user" for c in after["chats"] for m in c["messages"])
        == sum(m["role"] == "user" for c in interaction["chats"] for m in c["messages"]),
    "定向更新只改变一个子结果": sum(digest(v) != digest(next_views[k])
        for k, v in views.items()) == 1,
    "定向更新保留 Main 页面和版本": page["mainPage"] == updated["space"]["mainPage"],
    "所有线程最终结束且没有未处理请求": not any(c.get("activeTurn") or c.get("requests")
        or c.get("error") for c in updated["chats"]),
}
save(case / "composition-checks.json", {
    "at": now(), "checks": checks,
    "scope": "Real state invariants only. See composition-browser.json and screenshots for UI observations.",
})
for label, passed in checks.items():
    print("PASS" if passed else "FAIL", label)
raise SystemExit(0 if all(checks.values()) else 1)
