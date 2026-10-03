#!/usr/bin/env python3
"""R3-08 acceptance: isolated cache freshness, opt-out and recovery via Rust CLI."""
import json
from contextlib import closing
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile

CORE = Path(__file__).resolve().parents[2] / "core"
CLI = CORE / "target/debug" / ("docs-search-cli.exe" if sys.platform == "win32" else "docs-search-cli")


def search(folder, env, query="alpha", index=False, fuzzy=False):
    command = [str(CLI), str(folder), query, "--extensions", "txt"]
    if index:
        command.append("--use-index")
    if fuzzy:
        command.append("--fuzzy-search")
    result = subprocess.run(command, env=env, capture_output=True, text=True, encoding="utf-8", check=True)
    events = [json.loads(line) for line in result.stdout.splitlines()]
    assert events[0]["type"] == "started" and events[-1]["type"] == "finished"
    assert events[-1]["reason"] == "completed"
    assert not any(event["type"] == "issue" for event in events)
    hits = [event["hit"] for event in events if event["type"] == "result"]
    assert events[-1]["counts"]["resultCount"] == len(hits)
    stats = next(event for event in events if event["type"] == "executionSummary")
    return hits, (stats["extractedFiles"], stats["reusedFiles"])


def main():
    subprocess.run(["cargo", "build", "--locked", "--manifest-path", str(CORE / "Cargo.toml"), "--bin", "docs-search-cli"], check=True)
    with tempfile.TemporaryDirectory(prefix="docs-search-index-v3-") as temporary:
        base = Path(temporary)
        folder = base / "documents"
        folder.mkdir()
        path = folder / "sample.txt"
        path.write_text("alpha\n", encoding="utf-8")
        env = os.environ.copy()
        env["LOCALAPPDATA"] = str(base / "cache")
        database = base / "cache/docs-search/search-index.sqlite3"
        direct, stats = search(folder, env)
        assert len(direct) == 1 and stats == (1, 0)
        assert not database.parent.exists()
        print("PASS index off creates no cache")

        indexed, stats = search(folder, env, index=True)
        assert indexed == direct and stats == (1, 0)
        reused, stats = search(folder, env, index=True)
        assert reused == direct and stats == (0, 1)
        print("PASS first index and reuse preserve results and extraction counts")

        before = database.read_bytes()
        before_time = database.stat().st_mtime_ns
        old_time = path.stat().st_mtime_ns
        path.write_text("bravo\n", encoding="utf-8")  # Same size, explicit mtime change.
        os.utime(path, ns=(old_time + 2_000_000_000, old_time + 2_000_000_000))
        current, stats = search(folder, env, "bravo")
        assert len(current) == 1 and stats == (1, 0)
        assert database.read_bytes() == before and database.stat().st_mtime_ns == before_time
        with closing(sqlite3.connect(database)) as connection, connection:
            assert connection.execute("SELECT text FROM units").fetchall() == [("alpha",)]
        print("PASS index off reads current source and leaves existing cache unchanged")

        updated, stats = search(folder, env, "bravo", index=True)
        assert updated == current and stats == (1, 0)
        assert search(folder, env, "bravo", index=True)[1] == (0, 1)
        path.write_text("longer alpha\n", encoding="utf-8")
        os.utime(path, ns=(old_time + 2_000_000_000, old_time + 2_000_000_000))
        assert search(folder, env, "alpha", index=True)[1] == (1, 0)
        print("PASS mtime-only and size-only changes invalidate cache")

        path.unlink()
        assert search(folder, env, index=True) == ([], (0, 0))
        with closing(sqlite3.connect(database)) as connection, connection:
            for table in ["files", "units", "grams", "tokens"]:
                assert connection.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0] == 0
        print("PASS removed files are pruned with their indexed units")

        path.write_text("alpha\n", encoding="utf-8")
        database.write_bytes(b"not a SQLite database")
        recovered, stats = search(folder, env, index=True)
        assert len(recovered) == 1 and stats == (1, 0)
        with closing(sqlite3.connect(database)) as connection, connection:
            assert connection.execute("PRAGMA integrity_check").fetchone() == ("ok",)
        print("PASS corrupt database is rebuilt without losing results")

        with closing(sqlite3.connect(database)) as connection, connection:
            connection.execute("PRAGMA user_version=1")
        assert search(folder, env, index=True)[1] == (1, 0)
        with closing(sqlite3.connect(database)) as connection, connection:
            assert connection.execute("PRAGMA user_version").fetchone() == (2,)
        print("PASS old schema is rebuilt from source")

        path.write_text("a_\nalphabet_\n__\n", encoding="utf-8")
        for query in ["a!", "alpha!", "__"]:
            direct, _ = search(folder, env, query, fuzzy=True)
            assert len(direct) == 1, (query, direct)
            for _ in range(2):
                indexed, _ = search(folder, env, query, index=True, fuzzy=True)
                assert indexed == direct, (query, indexed, direct)
        print("PASS fuzzy token and separator candidates preserve direct/index equivalence")

        path.write_text("alpha\n", encoding="utf-8")

        blocked = base / "blocked-cache"
        blocked.write_bytes(b"cannot create a directory here")
        env["LOCALAPPDATA"] = str(blocked)
        assert len(search(folder, env, index=True)[0]) == 1
        assert search(folder, env, index=True)[1] == (1, 0)
        print("PASS unavailable cache falls back to direct extraction")


if __name__ == "__main__":
    main()
