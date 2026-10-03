#!/usr/bin/env python3
"""CLI acceptance for visible Excel text, position order, timestamps and index upgrades."""
import importlib.util
from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
CORE = ROOT / 'core'
CLI = CORE / 'target/debug' / ('docs-search-cli.exe' if sys.platform == 'win32' else 'docs-search-cli')


def search(folder, env, query, flags=(), conditions=None):
    command = [str(CLI), str(folder), query, '--extensions', 'xlsx', *flags]
    if conditions is not None:
        command += ['--query-spec-json', json.dumps(conditions, ensure_ascii=False)]
    result = subprocess.run(command, env=env, capture_output=True, text=True, encoding='utf-8')
    if result.returncode:
        raise AssertionError(result.stderr)
    events = [json.loads(line) for line in result.stdout.splitlines()]
    assert events[-1]['type'] == 'finished' and events[-1]['reason'] == 'completed'
    assert not [event for event in events if event['type'] == 'issue']
    return events, [event['hit'] for event in events if event['type'] == 'result']


def main():
    subprocess.run(['cargo', 'build', '--manifest-path', str(CORE / 'Cargo.toml'), '--bin', 'docs-search-cli'], check=True)
    spec = importlib.util.spec_from_file_location('issue_fixture', ROOT / 'tests/fixtures/generate-issue-fixture.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    with tempfile.TemporaryDirectory(prefix='docs-search-issues-') as temporary:
        base = Path(temporary)
        module.generate(base / 'fixture')
        folder = base / 'fixture/excel'
        path = folder / 'layout.xlsx'
        os.utime(path, (1_700_000_000, 1_700_000_000))
        env = os.environ.copy()
        env['LOCALAPPDATA'] = str(base / 'local-app-data')
        previous = None
        for flags in [(), ('--use-index',), ('--use-index',)]:
            events, hits = search(folder, env, 'ORDER', flags)
            ranking = next(event['ranking'] for event in events if event['type'] == 'fileRanked')
            by_id = {hit['resultId']: hit for hit in hits}
            locations = [(by_id[number]['location']['sheetName'], by_id[number]['location']['cellAddress']) for number in ranking['resultIds']]
            assert locations == [('Zeta', 'A2'), ('Zeta', 'Z2'), ('Zeta', 'AA2'), ('Zeta', 'A10'), ('Alpha', 'A1')], locations
            assert all(hit['modifiedAt'] == 1_700_000_000_000 for hit in hits)
            assert all('PHONETIC' not in hit['previewText'] for hit in hits)
            stable = sorted([{key: value for key, value in hit.items() if key != 'resultId'} for hit in hits], key=lambda hit: hit['documentOrder'])
            if previous is not None:
                assert stable == previous
            previous = stable
            for query in ['PHONETIC_ONLY', 'INLINE_PHONETIC_ONLY', 'RICH_PHONETIC_ONLY']:
                for fuzzy in [(), ('--fuzzy-search',)]:
                    _, hidden = search(folder, env, query, (*flags, *fuzzy))
                    assert not hidden, (query, flags, fuzzy)
                    conditions = {'mode': 'conditions', 'scope': 'unit', 'all': [query], 'any': [], 'not': []}
                    _, hidden = search(folder, env, '', (*flags, *fuzzy), conditions)
                    assert not hidden, (query, 'conditions', flags, fuzzy)
            _, visible = search(folder, env, '顧客', flags)
            assert len(visible) == 3
            for hit in visible:
                first, last = hit['matchRanges'][0]
                assert hit['previewText'][first:last] == '顧客'
            print(f'PASS visible text, workbook/numeric order, timestamp: {flags}')
        database = base / 'local-app-data/docs-search/search-index.sqlite3'
        other = base / 'other-root'
        module.workbook(other / 'archived.xlsx')
        search(other, env, 'ORDER', ('--use-index',))
        with closing(sqlite3.connect(database)) as connection, connection:
            # Simulate an old extraction cache containing a hidden phonetic term.
            connection.execute('UPDATE files SET extraction_version=3')
            connection.execute("UPDATE units SET text=text || ' PHONETIC_ONLY', loose=loose || ' phonetic_only'")
        _, hidden = search(folder, env, 'PHONETIC_ONLY', ('--use-index',))
        assert not hidden
        with closing(sqlite3.connect(database)) as connection, connection:
            assert connection.execute('SELECT DISTINCT extraction_version FROM files').fetchall() == [(4,)]
            assert connection.execute("SELECT COUNT(*) FROM units WHERE text LIKE '%PHONETIC_ONLY%'").fetchone()[0] == 0
            assert connection.execute('SELECT COUNT(*) FROM files WHERE path=?', (str(other / 'archived.xlsx'),)).fetchone()[0] == 0
        print('PASS extraction v3 cache rebuilt as v4 without phonetic metadata')


if __name__ == '__main__':
    main()
