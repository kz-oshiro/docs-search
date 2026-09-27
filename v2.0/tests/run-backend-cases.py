#!/usr/bin/env python3
"""Run the shared stack-independent backend cases against the v2.0 CLI."""

import json
import subprocess
import sys
import tempfile
import unicodedata
from pathlib import Path

HERE = Path(__file__).resolve().parent
DOC_SEARCH = HERE.parent.parent
CORE = HERE.parent / "core"
EXE = CORE / "target" / "debug" / ("doc-search-cli.exe" if sys.platform == "win32" else "doc-search-cli")


def check(condition, message):
    if not condition:
        raise AssertionError(message)


def main():
    subprocess.run(["cargo", "build", "--manifest-path", str(CORE / "Cargo.toml")], check=True)
    spec = json.loads((DOC_SEARCH / "tests" / "backend-cases.json").read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory(prefix="doc-search-v2.0-") as temp:
        root = Path(temp) / "fixture"
        subprocess.run([sys.executable, str(DOC_SEARCH / "tests" / "generate-fixtures.py"), "--output", str(root), "--profile", "load"], check=True)
        for case in spec["cases"]:
            request = case.get("request") or case["steps"][0]["request"]
            folder = request["rootDirectory"].replace("${fixtureRoot}", str(root))
            args = [str(EXE), folder, request["query"]]
            if "extensions" in request:
                args.extend(["--extensions", ",".join(request["extensions"])])
            if case["id"] == "cancel-load-search":
                args.append("--cancel-on-start")
            run = subprocess.run(args, text=True, encoding="utf-8", capture_output=True)
            events = [json.loads(line) for line in run.stdout.splitlines()]
            expected = case["expected"]
            prefix = case["id"]
            if not expected["accepted"]:
                check(run.returncode == 2 and not events and expected["inputErrorField"] in run.stderr, f"{prefix}: rejection")
                print(f"PASS {prefix}")
                continue
            check(run.returncode == 0, f"{prefix}: {run.stderr}")
            check(events[0]["type"] == "started" and events[-1]["type"] == "finished", f"{prefix}: boundaries")
            check(events[0]["request"]["extensions"] == request.get("extensions", ["xlsx", "xlsm", "pptx", "docx", "txt"]), f"{prefix}: selected extensions")
            check(sum(e["type"] == "started" for e in events) == 1 and sum(e["type"] == "finished" for e in events) == 1, f"{prefix}: terminal count")
            check([e["sequence"] for e in events] == list(range(1, len(events) + 1)), f"{prefix}: sequence")
            check(all(e["searchId"] == "cli" for e in events), f"{prefix}: search ID")
            final = events[-1]
            check(final["reason"] == expected["finished"], f"{prefix}: reason {final['reason']}")
            results = [e["hit"] for e in events if e["type"] == "result"]
            issues = [e["issue"] for e in events if e["type"] == "issue"]
            needle = unicodedata.normalize("NFC", request["query"].strip()).casefold()
            for hit in results:
                characters = list(hit["previewText"])
                ranges = hit["matchRanges"]
                check(bool(ranges), f"{prefix}: missing highlight range")
                previous_end = 0
                for start, end in ranges:
                    check(previous_end <= start < end <= len(characters), f"{prefix}: invalid highlight range")
                    marked = unicodedata.normalize("NFC", "".join(characters[start:end])).casefold()
                    check(needle in marked, f"{prefix}: highlight does not match query")
                    previous_end = end
            counts = final["counts"]
            check(counts["resultCount"] == len(results) and counts["issueCount"] == len(issues), f"{prefix}: event counts")
            check(counts["processedFiles"] <= counts["discoveredFiles"], f"{prefix}: processed count")
            if case["id"] == "cancel-load-search":
                print(f"PASS {prefix}")
                continue
            check(counts == expected["counts"], f"{prefix}: counts {counts} != {expected['counts']}")
            check(len(results) == len(expected["results"]), f"{prefix}: result number")
            check(len(issues) == len(expected["issues"]), f"{prefix}: issue number")
            def relative(path):
                return Path(path).relative_to(root / "search").as_posix()
            def hit_key(hit):
                return (relative(hit["filePath"]), hit["sourceKind"], json.dumps(hit["location"], sort_keys=True))
            actual_hits = {hit_key(hit): hit for hit in results}
            check(len(actual_hits) == len(results), f"{prefix}: duplicate results")
            for wanted in expected["results"]:
                key = (wanted["file"], wanted["sourceKind"], json.dumps(wanted["location"], sort_keys=True))
                check(key in actual_hits and wanted["textContains"] in actual_hits[key]["previewText"], f"{prefix}: missing {wanted}")
            actual_issues = {(relative(issue["path"]), issue["stage"], issue["code"]) for issue in issues}
            wanted_issues = {(issue["file"], issue["stage"], issue["code"]) for issue in expected["issues"]}
            check(actual_issues == wanted_issues, f"{prefix}: issues {actual_issues}")
            print(f"PASS {prefix}")


if __name__ == "__main__":
    main()
