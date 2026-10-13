"""Run one real case or prepare ten visible Spaces; evidence remains inspectable."""
import argparse
import json
from pathlib import Path
from client import OUTPUT, capture, prepare, save, send, wait

parser = argparse.ArgumentParser()
parser.add_argument("case", help="C01-C10, D01-D10 or prepare")
parser.add_argument("--suite", choices=["lifecycle", "visual"], default="lifecycle")
parser.add_argument("--prompt")
parser.add_argument("--view")
parser.add_argument("--label", default="baseline")
parser.add_argument("--wait-only", action="store_true")
args = parser.parse_args()
manifest = "visual-cases.json" if args.suite == "visual" else "cases.json"
cases = json.loads(Path(__file__).with_name(manifest).read_text())
if args.case == "prepare":
    for case in cases:
        print(str(prepare(case)), flush=True)
else:
    case = next(case for case in cases if case["id"] == args.case)
    cwd = OUTPUT / (case["id"] + "-" + case["title"])
    if not cwd.exists():
        cwd = prepare(case)
    if not args.wait_only:
        capture(cwd, args.label + "-before")
        print(send(cwd, args.prompt or case["prompt"], view=args.view), flush=True)
    result = wait(cwd, args.label)
    save(cwd / "evidence" / (args.label + "-summary.json"), result)
    print(json.dumps(result, ensure_ascii=False, indent=2), flush=True)
