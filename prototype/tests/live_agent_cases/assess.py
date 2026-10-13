"""Check captured real runs; this does not simulate or mark UI checks as passed."""
import json
from client import OUTPUT, api, capture, digest, now, save


def folder(case):
    return next(OUTPUT.glob(case + "-*"))


def read(case, label):
    return json.loads((folder(case) / "evidence" / (label + ".json")).read_text())


def view(snapshot, title):
    return next(v for v in snapshot["space"]["views"] if title in v["title"])


def messages(snapshot, role, author="user"):
    return [m["text"] for c in snapshot["chats"] if c["role"] == role
            for m in c["messages"] if m["role"] == author]


checks = []


def check(case, label, passed):
    checks.append({"case": case, "check": label, "passed": bool(passed)})


final = {f"C{i:02}": capture(folder(f"C{i:02}"), "final") for i in range(1, 11)}
c01 = final["C01"]
check("C01", "普通问答未创建 View 或 Main Page", not c01["space"]["views"]
      and not c01["space"]["mainPage"]["html"])
check("C01", "真实 Codex 已返回答案", any("召回" in m and "排序" in m
      for m in messages(c01, "main", "assistant")))
c02 = final["C02"]
check("C02", "Main Page 与两个独立可用 View", bool(c02["space"]["mainPage"]["html"])
      and len(c02["space"]["views"]) == 2
      and all(v["status"] == "ready" and v["html"] and v["threadId"] for v in c02["space"]["views"]))

b, a = read("C03", "targeted-update-before"), final["C03"]
old, new = view(b, "架构"), view(a, "架构")
check("C03", "未涉及架构的 HTML、版本、布局和线程不变", all(old[k] == new[k]
      for k in ("html", "version", "id", "threadId", "x", "y", "width", "height")))
old, new = view(b, "指标"), view(a, "指标")
check("C03", "指标页复用 ID/Thread 并实际更新 HTML", old["id"] == new["id"]
      and old["threadId"] == new["threadId"] and digest(old) != digest(new))

b, a = read("C04", "event-before"), read("C04", "event-after")
check("C04", "页面事件只更新所属 View", digest(b["space"]["mainPage"]) == digest(a["space"]["mainPage"])
      and digest(b["space"]["views"][0]) != digest(a["space"]["views"][0]))
check("C04", "一次点击仅产生一个 View 用户事件", len(messages(a, "view")) - len(messages(b, "view")) == 1
      and len(messages(a, "main")) == len(messages(b, "main")))

b, a = read("C05", "delete-update-before"), final["C05"]
old, new = view(b, "结果"), view(a, "结果")
deleted = view(b, "旧方案")
check("C05", "显式删除 View 和连接并清理页面目录", len(a["space"]["views"]) == 1
      and not a["space"]["connections"]
      and not (folder("C05") / ".space" / "views" / deleted["id"]).exists())
check("C05", "保留结果 View ID/Thread/HTML", old["id"] == new["id"]
      and old["threadId"] == new["threadId"] and digest(old) == digest(new))
b = read("C05", "layout-only-retest-before")
check("C05", "布局复测不启动 View 或改变版本", view(b, "结果")["version"] == new["version"]
      and len(messages(b, "view")) == len(messages(a, "view")) and new["x"] == 1280)

c06 = final["C06"]
check("C06", "三个独立 View 完成", len(c06["space"]["views"]) == 3
      and len({v["threadId"] for v in c06["space"]["views"]}) == 3
      and all(v["status"] == "ready" and v["html"] for v in c06["space"]["views"]))
events = [json.loads(line) for line in (folder("C06") / "evidence/events.jsonl").read_text().splitlines()]
check("C06", "真实命令进度和逐个交付等待状态", any("sleep 25" in str(e.get("detail")) for e in events)
      and all(any(f"正在等待 View · {count} 个" == e.get("activity") for e in events) for count in (3, 2, 1)))

c07 = read("C07", "queue-order-final")
queued = messages(c07, "view")
c_index = next(i for i, text in enumerate(queued) if text.startswith("排队 C 已编辑"))
b_index = next(i for i, text in enumerate(queued) if text.startswith("排队 B"))
check("C07", "拖动与编辑后的 C 先于 B 执行，D 已删除", c_index < b_index
      and not any(text.startswith("排队 D") for text in queued))
dedup = read("C07", "dedup")
check("C07", "重复 requestId 未重复执行", dedup["userMessageCopies"] == 1
      and dedup["messagesBefore"] == dedup["messagesAfter"])
background = read("C07", "background-delivery-final")
check("C07", "切走 Space 后仍完成排队", "已在其他 Space 打开时完成排队"
      in messages(background, "main", "assistant"))
check("C07", "停止后保留旧 HTML，未发布迟到结果", digest(read("C07", "stop-check-before")["space"]["views"][0])
      == digest(read("C07", "stop-verified")["space"]["views"][0])
      and "不应出现的停止后结果" not in final["C07"]["space"]["views"][0]["html"])

b, a = read("C08", "missing-data-before"), read("C08", "missing-data-verified")
check("C08", "缺失文件失败保留已有 HTML", a["space"]["views"][0]["status"] == "error"
      and digest(b["space"]["views"][0]) == digest(a["space"]["views"][0]))
v08 = final["C08"]["space"]["views"][0]
check("C08", "同一线程重试后交付夹具数据页面", v08["status"] == "ready"
      and v08["threadId"] == a["space"]["views"][0]["threadId"]
      and digest(v08) != digest(a["space"]["views"][0]))
c09 = final["C09"]
check("C09", "批准后的命令结果实际交付", any("退出码 0" in m and "C09_APPROVED" in m
      for m in messages(c09, "view", "assistant")) and c09["space"]["views"][0]["status"] == "ready")
check("C09", "未取得主题回答时保留已有页面", digest(read("C09", "user-input-before")["space"]["views"][0])
      == digest(c09["space"]["views"][0]))

restart = read("C10", "restart-checks")
check("C10", "服务重启恢复 Main/View ID 和 HTML", all(restart[k] for k in
      ("sameMainThread", "sameViewId", "sameViewThread", "htmlRetained")))
v10 = final["C10"]["space"]["views"][0]
check("C10", "中断提示明确且原线程继续交付 v2", any(c["error"] and "上次服务退出" in c["error"]
      for c in restart["chat"]) and v10["status"] == "ready" and "恢复测试 v2" in v10["html"]
      and v10["threadId"] == read("C10", "restart-restored")["space"]["views"][0]["threadId"])
check("ALL", "十个 Space 当前没有悬挂任务或请求", not any(c.get("activeTurn") or c.get("requests")
      for snap in final.values() for c in snap["chats"]))

report = {"at": now(), "type": "real-run-state-invariants", "checks": checks,
          "note": "These checks do not assert complete product acceptance; see LIVE_AGENT_CASE_RESULTS.md for observed gaps."}
save(OUTPUT / "verification.json", report)
for item in checks:
    print(("PASS" if item["passed"] else "FAIL"), item["case"], item["check"])
print(f"{sum(c['passed'] for c in checks)}/{len(checks)} recorded state checks passed")
raise SystemExit(0 if all(c["passed"] for c in checks) else 1)
