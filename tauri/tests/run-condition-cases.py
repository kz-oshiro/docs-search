#!/usr/bin/env python3
"""Check P3 conditions through the CLI with a separate generated fixture."""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CORE = ROOT / "tauri" / "core"
CLI = CORE / "target" / "debug" / ("docs-search-cli.exe" if sys.platform == "win32" else "docs-search-cli")


def check(condition, label):
    if not condition:
        raise AssertionError(label)


def condition(scope, all=(), any=(), not_terms=()):
    return {"mode": "conditions", "scope": scope, "all": list(all), "any": list(any), "not": list(not_terms)}


def run(root, env, spec=None, *, query="", extensions="xlsx", folder="search", indexed=False, fuzzy=False):
    args = [str(CLI), str(root / folder), query, "--extensions", extensions]
    if spec is not None:
        args += ["--query-spec-json", json.dumps(spec, ensure_ascii=False)]
    if indexed:
        args.append("--use-index")
    if fuzzy:
        args.append("--fuzzy-search")
    result = subprocess.run(args, text=True, encoding="utf-8", capture_output=True, env=env)
    events = [json.loads(line) for line in result.stdout.splitlines()]
    if result.returncode != 0:
        return result, events, [], []
    check(events[0]["type"] == "started" and events[-1]["type"] == "finished", "event boundaries")
    check([event["sequence"] for event in events] == list(range(1, len(events) + 1)), "event sequence")
    hits = [event["hit"] for event in events if event["type"] == "result"]
    issues = [event["issue"] for event in events if event["type"] == "issue"]
    counts = events[-1]["counts"]
    check(events[-1]["reason"] == "completed", "search completion")
    check(counts["resultCount"] == len(hits) and counts["issueCount"] == len(issues), "event counts")
    return result, events, hits, issues


def places(hits):
    return sorted((Path(hit["filePath"]).name, hit["sourceKind"],
                   hit["location"].get("sheetName"), hit["location"].get("row"),
                   hit["location"].get("cellAddress"), hit["location"].get("lineNumber"))
                  for hit in hits)


def evidence(hit):
    return {item["term"]: item["location"].get("cellAddress") for item in hit["evidence"]}


def accepted(root, env, name, spec, expected, **kwargs):
    result, _, hits, issues = run(root, env, spec, **kwargs)
    check(result.returncode == 0 and not issues, f"{name}: {result.stderr or issues}")
    check(places(hits) == expected, f"{name}: {places(hits)} != {expected}")
    print(f"PASS {name}")
    return hits


def main():
    subprocess.run(["cargo", "build", "--manifest-path", str(CORE / "Cargo.toml"), "--bin", "docs-search-cli"], check=True)
    with tempfile.TemporaryDirectory(prefix="docs-search-conditions-") as temporary:
        base = Path(temporary)
        fixture = base / "fixture"
        subprocess.run([sys.executable, str(ROOT / "tests" / "generate-conditions-fixture.py"),
                        "--output", str(fixture)], check=True)
        env = os.environ.copy()
        env["LOCALAPPDATA"] = str(base / "local-app-data")

        row = condition("excelRow", all=("顧客", "必須"))
        row_places = [("conditions.xlsx", "excelRow", "Items", 12, None, None),
                      ("conditions.xlsx", "excelRow", "Items", 15, None, None)]
        hits = accepted(fixture, env, "row AND", row, row_places)
        check(evidence(hits[0]) == {"顧客": "A12", "必須": "D12"}, "row 12 evidence")
        check(evidence(hits[1]) == {"顧客": "A15", "必須": "D15"}, "row 15 evidence")
        accepted(fixture, env, "unit boundary", condition("unit", all=("顧客", "必須")), [])
        file_places = [("conditions.xlsx", "fileMatch", None, None, None, None),
                       ("notes.txt", "fileMatch", None, None, None, None)]
        accepted(fixture, env, "file AND", condition("file", all=("顧客", "必須")),
                 file_places, extensions="xlsx,txt")

        excluded = condition("excelRow", all=("顧客", "必須"), not_terms=("廃止",))
        accepted(fixture, env, "row NOT", excluded, row_places[:1])
        or_places = [("conditions.xlsx", "excelRow", "Items", number, None, None)
                     for number in (12, 13, 14, 15)]
        or_places += [("conditions.xlsx", "excelRow", "Other", 12, None, None)]
        or_places.sort()
        accepted(fixture, env, "row OR", condition("excelRow", any=("顧客", "必須")), or_places)

        name_body = condition("file", all=("filename-signal", "body-signal"))
        accepted(fixture, env, "filename and body", name_body,
                 [("filename-signal.xlsx", "fileMatch", None, None, None, None)])
        for scope in ("unit", "excelRow"):
            accepted(fixture, env, f"filename boundary {scope}",
                     condition(scope, all=("filename-signal", "body-signal")), [])

        literal = [("conditions.xlsx", "cell", "Items", None, "B16", None),
                   ("notes.txt", "textLine", None, None, None, 3)]
        accepted(fixture, env, "literal advanced", condition("unit", all=("AND OR",)),
                 literal, extensions="xlsx,txt")
        accepted(fixture, env, "literal normal", None, literal, query="AND OR", extensions="xlsx,txt")

        typo = [("conditions.xlsx", "cell", "Items", None, "A17", None)]
        accepted(fixture, env, "typo off", condition("unit", all=("custmer",)), [])
        fuzzy_hits = accepted(fixture, env, "typo on", condition("unit", all=("custmer",)), typo, fuzzy=True)
        check(fuzzy_hits[0]["matchCategory"] == "fuzzy", "typo match category")
        accepted(fixture, env, "NOT stays literal",
                 condition("unit", all=("customer",), not_terms=("custmer",)), typo, fuzzy=True)

        for spec, expected, label in ((row, row_places, "row AND"),
                                      (excluded, row_places[:1], "row NOT"),
                                      (condition("file", all=("顧客", "必須")), file_places, "file AND")):
            for indexed in (False, True):
                for fuzzy in (False, True):
                    accepted(fixture, env, f"{label} index={indexed} fuzzy={fuzzy}", spec,
                             expected, extensions="xlsx,txt" if label == "file AND" else "xlsx",
                             indexed=indexed, fuzzy=fuzzy)

        for name, spec, query in (
            ("positive terms required", condition("unit", not_terms=("A",)), ""),
            ("all and NOT conflict", condition("unit", all=("A",), not_terms=("a",)), ""),
            ("unknown scope", condition("unknown", all=("A",)), ""),
            ("term too long", condition("unit", all=("A" * 201,)), ""),
            ("normal and advanced mixed", condition("unit", all=("A",)), "A"),
        ):
            result, events, _, _ = run(fixture, env, spec, query=query)
            check(result.returncode == 2 and not events and "querySpec" in result.stderr,
                  f"{name}: expected querySpec error, got {result.returncode} {result.stderr}")
            print(f"PASS {name}")

        result, _, hits, issues = run(fixture, env, name_body, folder="errors")
        check(result.returncode == 0 and not hits and len(issues) == 1 and issues[0]["stage"] == "read",
              "unreadable file scope")
        print("PASS unreadable file scope")


if __name__ == "__main__":
    main()
