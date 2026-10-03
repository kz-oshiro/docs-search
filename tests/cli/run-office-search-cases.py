#!/usr/bin/env python3
"""P4-P6 CLI acceptance, index equivalence, batch completeness, and 20 ranking pairs."""
import argparse
from contextlib import closing
import json
import os
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CORE = ROOT / "core"
CLI = CORE / "target" / "debug" / ("docs-search-cli.exe" if sys.platform == "win32" else "docs-search-cli")


def check(value, label):
    if not value: raise AssertionError(label)


def run(root, env, query="", spec=None, flags=(), *, extensions="xlsx,pptx,docx", accepted=True, cancelled=False):
    args = [str(CLI), str(root), query, "--extensions", extensions, *flags]
    if spec is not None: args += ["--query-spec-json", json.dumps(spec, ensure_ascii=False)]
    output = subprocess.run(args, text=True, encoding="utf-8", capture_output=True, env=env)
    events = [json.loads(line) for line in output.stdout.splitlines()]
    if not accepted:
        check(output.returncode == 2 and not events, f"input rejection: {output.stderr}")
        return [], [], [], []
    check(output.returncode == 0, output.stderr)
    check(events[0]["type"] == "started" and events[-1]["type"] == "finished", "event boundaries")
    check([item["sequence"] for item in events] == list(range(1,len(events)+1)), "sequence")
    check(sum(item["type"] == "finished" for item in events) == 1, "single terminal")
    hits = [item["hit"] for item in events if item["type"] == "result"]
    issues = [item["issue"] for item in events if item["type"] == "issue"]
    counts = events[-1]["counts"]
    check(counts["resultCount"] == len(hits) and counts["issueCount"] == len(issues), "result/issue counts")
    check(counts["processedFiles"] <= counts["discoveredFiles"], "file counts")
    check(events[-1]["reason"] == ("cancelled" if cancelled else "completed"), "terminal reason")
    rankings = [item["ranking"] for item in events if item["type"] == "fileRanked"]
    for ranking in rankings:
        check(set(ranking["resultIds"]) == {hit["resultId"] for hit in hits if hit["filePath"]==ranking["filePath"]}, "ranked IDs")
    return events, hits, issues, rankings


def signature(hits):
    return sorted((Path(hit["filePath"]).name, hit["sourceKind"], json.dumps(hit["location"],sort_keys=True), hit["previewText"], hit["matchType"],hit.get("termId","")) for hit in hits)


def summary(events, kind):
    matches = [event for event in events if event["type"] == kind]
    check(len(matches)==1, f"single {kind}")
    return matches[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, help="Save ranking Top-5 comparison without overwriting an existing file")
    args = parser.parse_args()
    if args.report and args.report.exists(): parser.error("Report already exists")
    subprocess.run(["cargo","build","--manifest-path",str(CORE/"Cargo.toml"),"--bin","docs-search-cli"], check=True)
    with tempfile.TemporaryDirectory(prefix="docs-search-office-") as temporary:
        base=Path(temporary); fixture=base/"fixture"
        subprocess.run([sys.executable,str(ROOT/"tests" / "fixtures" / "generate-office-search-fixture.py"),"--output",str(fixture)],check=True)
        env=os.environ.copy(); env["LOCALAPPDATA"]=str(base/"cache")
        search=fixture/"search"
        cases=[("FORMULA_SIGNAL","formula",1,"--include-formulas"), ("SHARED_SIGNAL","formula",1,"--include-formulas"),
               ("FORMULA_WITHOUT_CACHE","formula",1,"--include-formulas"), ("NOTE_SIGNAL","note",1,"--include-notes"),
               ("ROOT_SIGNAL","excelComment",1,"--include-notes"), ("REPLY_SIGNAL","excelComment",1,"--include-notes"),
               ("HEADER_SIGNAL","wordHeader",1,"--include-notes"), ("HEADER_TABLE_SIGNAL","wordHeader",1,"--include-notes"),
               ("FOOTER_SIGNAL","wordFooter",1,"--include-notes"), ("WORD_COMMENT_SIGNAL","wordComment",1,"--include-notes")]
        for query,kind,count,flag in cases:
            _,off,issues,_=run(search,env,query); check(not off and not issues,f"{query} defaults")
            expected=None
            for extra in ((),("--use-index",),("--use-index",)):
                _,hits,issues,_=run(search,env,query,flags=(flag,*extra))
                check(len(hits)==count and all(hit["sourceKind"]==kind for hit in hits) and not issues,f"{query} sources")
                if expected is None: expected=signature(hits)
                else: check(signature(hits)==expected,f"{query} cache equivalence")
        _,cached,_,_=run(search,env,"42",flags=("--include-formulas",))
        check(len(cached)==2 and {hit["sourceKind"] for hit in cached}=={"cell","formula"}
              and all(hit["location"]["cellAddress"]=="C1" for hit in cached),"saved value and formula text are separate at the same cell")
        for query in ("IGNORED_SIGNAL","ORPHAN_SIGNAL"):
            _,hits,issues,_=run(search,env,query,flags=("--include-notes",)); check(not hits and not issues,"placeholder/orphan exclusion")
        for flags,kind in (((),"shape"),(("--include-notes",),"excelComment")):
            _,hits,issues,_=run(search,env,"LEGACY_SIGNAL",flags=flags)
            check(len(hits)==1 and hits[0]["sourceKind"]==kind and not issues,"VML dedup and default compatibility")
        _,hits,_,_=run(search,env,"VML_SIGNAL",flags=("--include-notes",)); check(len(hits)==1 and hits[0]["sourceKind"]=="shape","normal VML remains")
        _,hits,_,_=run(fixture/"macros",env,"SHARED_SIGNAL",flags=("--include-formulas",),extensions="xlsm"); check(len(hits)==1,"xlsm formula")
        spec={"mode":"conditions","scope":"excelRow","all":["TAB_01","FORMULA_SIGNAL"],"any":[],"not":[]}
        _,hits,issues,_=run(search,env,spec=spec,flags=("--include-formulas",)); check(len(hits)==1 and hits[0]["sourceKind"]=="excelRow" and not issues,"formula row AND")
        for fault in ("malformed","missing","external","invalid-utf8","oversized","word-error"):
            events,hits,issues,_=run(fixture/fault,env,"BODY_SIGNAL",flags=("--include-notes","--use-index"))
            check(len(hits)==1 and len(issues)==1,"partial body retained / duplicate issue")
            file_spec={"mode":"conditions","scope":"file","all":["BODY_SIGNAL"],"any":[],"not":["unknown"]}
            _,hits,issues,_=run(fixture/fault,env,spec=file_spec,flags=("--include-notes",)); check(not hits and len(issues)==1,"incomplete file cannot satisfy NOT")
            database=base/"cache"/"docs-search"/"search-index.sqlite3"
            with closing(sqlite3.connect(database)) as db, db:
                rows=db.execute("SELECT path FROM files").fetchall()
                check(not any(str(fixture/fault) in path for (path,) in rows),"partial index not saved")
        for notes in (False,True):
            for formulas in (False,True):
                flags=tuple(flag for flag,enabled in (("--include-notes",notes),("--include-formulas",formulas)) if enabled)
                _,direct,_,_=run(search,env,"SIGNAL",flags=flags)
                _,indexed,_,_=run(search,env,"SIGNAL",flags=(*flags,"--use-index"))
                check(signature(direct)==signature(indexed),"scope switch equivalence")
        batch={"mode":"batch","terms":["", "TAB_01", "tab_01", "TAB_010", "MISSING_ID"],"matchMode":"identifier"}
        first=None
        for attempt,flags in enumerate(((),("--use-index",),("--use-index",))):
            events,hits,issues,_=run(search,env,spec=batch,flags=flags)
            terms=summary(events,"batchSummary")["terms"]
            check([term["term"] for term in terms]==["TAB_01","TAB_010","MISSING_ID"],"dedup input order")
            check([term["hitCount"] for term in terms]==[2,2,0],"identifier boundaries and hit counts")
            check([term["fileCount"] for term in terms]==[2,2,0],"term file counts")
            check(sum(term["hitCount"] for term in terms)==len(hits)
                  and all(hit["termId"] in {term["termId"] for term in terms} for hit in hits),"batch and global totals refer to the same terms")
            check(terms[-1]["status"]=="指定範囲内で該当なし" and not issues,"complete zero result")
            if flags:
                execution=summary(events,"executionSummary")
                expected_execution=(3,0) if attempt==1 else (0,3)
                check((execution["extractedFiles"],execution["reusedFiles"])==expected_execution,"scope refresh then one cache read per file")
            if first is None: first=signature(hits)
            else: check(signature(hits)==first,"batch cache equivalence")
        for terms in (["MISSING_ID"],[f"ID_{i}" for i in range(256)]):
            events,_,_,_=run(search,env,spec={"mode":"batch","terms":terms})
            execution=summary(events,"executionSummary")
            check(execution["extractedFiles"]==3 and execution["reusedFiles"]==0,"one extraction per file independent of term count")
        run(search,env,spec={"mode":"batch","terms":[f"ID_{i}" for i in range(257)]},accepted=False)
        run(search,env,spec={"mode":"batch","terms":["顧客"]},accepted=False)
        for folder,flags,status,cancelled in ((search,("--cancel-on-start",),"未確定（中断）",True),(fixture/"malformed",("--include-notes",),"未確定（一部エラーあり）",False)):
            events,_,_,_=run(folder,env,spec={"mode":"batch","terms":["MISSING_ID"]},flags=flags,cancelled=cancelled)
            check(summary(events,"batchSummary")["terms"][0]["status"]==status,"uncertain absence")
        text_batch={"mode":"batch","terms":["BODY_SIGNAL","TAB_01"],"matchMode":"text"}
        _,hits,_,_=run(search,env,spec=text_batch)
        check(len({hit["termId"] for hit in hits})==2,"text mode multiple terms in one unit")
        report=[]
        ranking_cases=json.loads((fixture/"ranking-cases.json").read_text(encoding="utf-8")); check(len(ranking_cases)==20,"fixed 20 queries")
        for case in ranking_cases:
            prior=None
            for cache_flags in ((),("--use-index",),("--use-index",)):
                events,hits,issues,rankings=run(fixture/case["folder"],env,case["query"],case.get("spec"),(*case["flags"],*cache_flags),extensions=case["extensions"])
                check(not issues,"ranking issue-free")
                order=[Path(path).name for path in summary(events,"rankingSummary")["files"]]
                check(case["prefer"] in order and case["over"] in order and order.index(case["prefer"]) < order.index(case["over"]),f"ranking pair {case['name']}: {order}")
                if prior is None: prior=order
                else: check(order==prior,"ranking index equivalence")
                if "firstAddress" in case:
                    ranking=next(item for item in rankings if Path(item["filePath"]).name==case["prefer"])
                    first_hit=next(hit for hit in hits if hit["resultId"]==ranking["resultIds"][0])
                    check(first_hit["location"]["cellAddress"]==case["firstAddress"],"numeric location order")
            groups={item["filePath"]:[hit for hit in hits if hit["filePath"]==item["filePath"]] for item in rankings}
            old=sorted(groups,key=lambda path:(-max(hit["score"] for hit in groups[path]),-int(any(hit["sourceKind"]=="fileName" for hit in groups[path])),path.casefold()))
            report.append({"case":case["name"],"query":case["query"],"oldTop5":[Path(path).name for path in old[:5]],"newTop5":order[:5],"evidence":[item["evidence"] for item in rankings]})
            print(f"PASS {case['name']}: old={[Path(path).name for path in old[:5]]} new={order[:5]}")
        if args.report:
            args.report.parent.mkdir(parents=True,exist_ok=True)
            args.report.write_text(json.dumps(report,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
    print("P4-P6 CLI acceptance and 20 ranking pairs passed.")


if __name__ == "__main__": main()
