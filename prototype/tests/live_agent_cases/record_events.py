"""Capture real SSE activity for these test Spaces only, without service tokens."""
import json
import time
import urllib.request
from client import BASE, OUTPUT, now

cursor = 0
while True:
    try:
        health = json.load(urllib.request.urlopen(BASE + "/health", timeout=10))
        request = urllib.request.Request(BASE + "/api/events?since=" + str(cursor),
                                         headers={"X-Chat-UI-Token": health["token"]})
        with urllib.request.urlopen(request, timeout=30) as response:
            for line in response:
                if not line.startswith(b"data: "):
                    continue
                event = json.loads(line[6:])
                cursor = event.get("sequence", cursor)
                params = event.get("params", {})
                thread_id = params.get("threadId") or params.get("thread", {}).get("id")
                for chat in event.get("state", {}).get("chats", []):
                    cwd = chat.get("cwd", "")
                    if not cwd.startswith(str(OUTPUT) + "/") or chat["id"] != thread_id:
                        continue
                    entry = {"at": now(), "sequence": cursor, "method": event.get("method"),
                             "threadId": chat["id"], "role": chat.get("role"),
                             "viewId": chat.get("viewId"), "activeTurn": chat.get("activeTurn"),
                             "activity": chat.get("activity"), "detail": chat.get("activityDetail"),
                             "error": chat.get("error"), "requestCount": len(chat.get("requests", []))}
                    path = OUTPUT / cwd.split("/")[-1] / "evidence" / "events.jsonl"
                    with path.open("a") as stream:
                        stream.write(json.dumps(entry, ensure_ascii=False) + "\n")
    except (OSError, ValueError) as error:
        print(type(error).__name__, flush=True)
        time.sleep(2)
        cursor = 0
