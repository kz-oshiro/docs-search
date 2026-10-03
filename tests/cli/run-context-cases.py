#!/usr/bin/env python3
"""Check P2 Excel search locations and indexed extraction through the CLI."""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CORE = ROOT / "core"
CLI = CORE / "target" / "debug" / ("docs-search-cli.exe" if sys.platform == "win32" else "docs-search-cli")

CASES = {
    "context-needle": ("cell", "B12", 12, 2, None),
    "edge-top": ("cell", "A1", 1, 1, None),
    "edge-bottom": ("cell", "XFD1048576", 1048576, 16384, None),
    "context-shape": ("shape", None, None, None, "B12"),
    "context-unanchored": ("shape", None, None, None, None),
}


def search(folder, env, query, indexed):
    args = [str(CLI), str(folder), query, "--extensions", "xlsx"]
    if indexed:
        args.append("--use-index")
    result = subprocess.run(args, text=True, encoding="utf-8", capture_output=True, env=env)
    if result.returncode:
        raise AssertionError(f"{query} index={indexed}: {result.stderr}")
    events = [json.loads(line) for line in result.stdout.splitlines()]
    hits = [event["hit"] for event in events if event["type"] == "result"]
    issues = [event for event in events if event["type"] == "issue"]
    if (len(hits), len(issues), events[-1]["counts"]["resultCount"]) != (1, 0, 1):
        raise AssertionError(f"{query} index={indexed}: unexpected hits/issues/counts")
    hit = hits[0]
    actual = (hit["sourceKind"], hit["location"].get("cellAddress"), hit.get("row"),
              hit.get("column"), hit.get("anchor"))
    if actual != CASES[query]:
        raise AssertionError(f"{query} index={indexed}: {actual} != {CASES[query]}")
    if hit["location"].get("sheetName") != "Context":
        raise AssertionError(f"{query} index={indexed}: wrong sheet")
    return {key: value for key, value in hit.items() if key != "resultId"}


def main():
    subprocess.run(["cargo", "build", "--manifest-path", str(CORE / "Cargo.toml"), "--bin", "docs-search-cli"], check=True)
    with tempfile.TemporaryDirectory(prefix="docs-search-context-") as temporary:
        base = Path(temporary)
        folder = base / "fixture"
        subprocess.run([sys.executable, str(ROOT / "tests" / "fixtures" / "generate-context-fixture.py"),
                        "--output", str(folder / "row-window.xlsx")], check=True)
        env = os.environ.copy()
        env["LOCALAPPDATA"] = str(base / "local-app-data")
        for query in CASES:
            direct = search(folder, env, query, False)
            first_index = search(folder, env, query, True)
            reused_index = search(folder, env, query, True)
            if direct != first_index or direct != reused_index:
                raise AssertionError(f"{query}: indexed result differs from direct extraction")
            print(f"PASS {query} direct/index initial/index reused")


if __name__ == "__main__":
    main()
