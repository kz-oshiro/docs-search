#!/usr/bin/env python3
"""Run the shared stack-independent backend cases against the Tauri app's CLI."""

import json
import os
import re
import subprocess
import sys
import tempfile
import unicodedata
from html.parser import HTMLParser
from pathlib import Path

HERE = Path(__file__).resolve().parent
DOCS_SEARCH = HERE.parent.parent
CORE = HERE.parent / "core"
EXE = CORE / "target" / "debug" / ("docs-search-cli.exe" if sys.platform == "win32" else "docs-search-cli")


def check(condition, message):
    if not condition:
        raise AssertionError(message)


class ExtensionInputs(HTMLParser):
    def __init__(self):
        super().__init__()
        self.values = []
        self.defaults = []

    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        if tag == "input" and attributes.get("name") == "extension":
            value = attributes["value"]
            self.values.append(value)
            if "checked" in attributes:
                self.defaults.append(value)


def check_extension_catalog():
    source = (CORE / "src" / "lib.rs").read_text(encoding="utf-8")
    catalog = re.search(r"pub const SUPPORTED_EXTENSIONS:.*?=\s*&\[(.*?)\];", source, re.S)
    check(catalog is not None, "missing backend extension catalog")
    supported = re.findall(r'"([a-z0-9]+)"', catalog.group(1))
    inputs = ExtensionInputs()
    inputs.feed((HERE.parent / "frontend" / "index.html").read_text(encoding="utf-8"))
    check(len(supported) == len(set(supported)), "duplicate backend extension")
    check(len(inputs.values) == len(set(inputs.values)), "duplicate GUI extension")
    check(set(supported) == set(inputs.values), "GUI and backend extensions differ")
    check(inputs.defaults == inputs.values, "GUI default extensions must all be selected")


def main():
    check_extension_catalog()
    subprocess.run(["cargo", "build", "--manifest-path", str(CORE / "Cargo.toml")], check=True)
    spec = json.loads((DOCS_SEARCH / "tests" / "backend-cases.json").read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory(prefix="docs-search-tauri-") as temp:
        root = Path(temp) / "fixture"
        cli_env = os.environ.copy()
        cli_env["LOCALAPPDATA"] = str(Path(temp) / "local-app-data")
        subprocess.run([sys.executable, str(DOCS_SEARCH / "tests" / "generate-fixtures.py"), "--output", str(root), "--profile", "load"], check=True)
        for case in spec["cases"]:
            request = case.get("request") or case["steps"][0]["request"]
            folder = request["rootDirectory"].replace("${fixtureRoot}", str(root))
            args = [str(EXE), folder, request["query"]]
            for directory in request.get("additionalDirectories", []):
                args.extend(["--add-directory", directory.replace("${fixtureRoot}", str(root))])
            for directory in request.get("excludedDirectories", []):
                args.extend(["--exclude-directory", directory.replace("${fixtureRoot}", str(root))])
            if "extensions" in request:
                args.extend(["--extensions", ",".join(request["extensions"])])
            if request.get("useIndex"):
                args.append("--use-index")
            if request.get("fuzzySearch"):
                args.append("--fuzzy-search")
            if case["id"] == "cancel-load-search":
                args.append("--cancel-on-start")
            run = subprocess.run(args, text=True, encoding="utf-8", capture_output=True, env=cli_env)
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
            check(events[0]["request"]["useIndex"] == request.get("useIndex", False), f"{prefix}: index option")
            check(events[0]["request"]["fuzzySearch"] == request.get("fuzzySearch", False), f"{prefix}: fuzzy option")
            check(events[0]["request"]["additionalDirectories"] == [directory.replace("${fixtureRoot}", str(root)) for directory in request.get("additionalDirectories", [])], f"{prefix}: additional directories")
            check(events[0]["request"]["excludedDirectories"] == [directory.replace("${fixtureRoot}", str(root)) for directory in request.get("excludedDirectories", [])], f"{prefix}: excluded directories")
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
                    if hit["matchType"] in ("exact", "caseFolded"):
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
                if "matchType" in wanted:
                    check(actual_hits[key]["matchType"] == wanted["matchType"], f"{prefix}: match type {wanted}")
                if "matchCategory" in wanted:
                    check(actual_hits[key]["matchCategory"] == wanted["matchCategory"], f"{prefix}: match category {wanted}")
                if "score" in wanted:
                    check(actual_hits[key]["score"] == wanted["score"], f"{prefix}: score {wanted}")
            actual_issues = {(relative(issue["path"]), issue["stage"], issue["code"]) for issue in issues}
            wanted_issues = {(issue["file"], issue["stage"], issue["code"]) for issue in expected["issues"]}
            check(actual_issues == wanted_issues, f"{prefix}: issues {actual_issues}")
            print(f"PASS {prefix}")


if __name__ == "__main__":
    main()
