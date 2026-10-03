#!/usr/bin/env python3
"""Generate separate deterministic P4-P6 acceptance documents and ranking pairs."""
import argparse
import json
import runpy
from pathlib import Path
from zipfile import ZipFile, ZipInfo, ZIP_DEFLATED

COMMON = runpy.run_path(str(Path(__file__).with_name("generate-fixtures.py")))
R, P, S, W, A = (COMMON[key] for key in ("DOC_REL", "PKG_REL", "SHEET", "WORD", "DRAWING"))
PPT = COMMON["PRESENTATION"]
xml, cell = COMMON["xml"], COMMON["cell"]


def package(path, entries):
    path.parent.mkdir(parents=True, exist_ok=True)
    with ZipFile(path, "w") as archive:
        for name, content in entries.items():
            info = ZipInfo(name, date_time=(2020, 1, 1, 0, 0, 0))
            info.compress_type = ZIP_DEFLATED
            info.create_system = 0
            archive.writestr(info, content.encode("utf-8") if isinstance(content, str) else content)


def rels(items):
    return f'<Relationships xmlns="{P}">' + ''.join(
        f'<Relationship Id="{id}" Type="{R}/{kind}" Target="{xml(target)}"'
        + (' TargetMode="External"' if external else '') + '/>'
        for id, kind, target, external in items) + '</Relationships>'


def base(main, kind):
    return {"_rels/.rels": rels([("root", "officeDocument", main, False)]),
            "[Content_Types].xml": f'<Types xmlns="{COMMON["CONTENT_TYPES"]}"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/{main}" ContentType="{kind}"/></Types>'}


def workbook(rows, comments=False, macro=False):
    main_type = "application/vnd.ms-excel.sheet.macroEnabled.main+xml" if macro else "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
    entries = base("xl/workbook.xml", main_type)
    entries["xl/workbook.xml"] = f'<workbook xmlns="{S}" xmlns:r="{R}"><sheets><sheet name="Items" sheetId="1" r:id="sheet"/></sheets></workbook>'
    entries["xl/_rels/workbook.xml.rels"] = rels([("sheet", "worksheet", "worksheets/data.xml", False)])
    entries["xl/worksheets/data.xml"] = f'<worksheet xmlns="{S}" xmlns:r="{R}"><sheetData>{rows}</sheetData>' + ('<legacyDrawing r:id="vml"/>' if comments else '') + '</worksheet>'
    if comments:
        entries["xl/worksheets/_rels/data.xml.rels"] = rels([
            ("vml", "vmlDrawing", "../drawings/shapes.vml", False),
            ("comment", "comments", "../extra/legacy.xml", False),
            ("thread", "threadedComment", "../extra/thread.xml", False)])
        entries["xl/extra/legacy.xml"] = f'<comments xmlns="{S}"><commentList><comment ref="A1" authorId="0"><text><t>LEGACY_SIGNAL</t></text></comment></commentList></comments>'
        entries["xl/extra/thread.xml"] = '<ThreadedComments xmlns="http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments"><threadedComment ref="B1" id="root"><text>ROOT_SIGNAL</text></threadedComment><threadedComment parentId="root" id="reply"><text>REPLY_SIGNAL</text></threadedComment></ThreadedComments>'
        entries["xl/drawings/shapes.vml"] = '<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:x="urn:schemas-microsoft-com:office:excel"><v:shape id="comment"><v:textbox><div>LEGACY_SIGNAL</div></v:textbox><x:ClientData ObjectType="Note"><x:Row>0</x:Row><x:Column>0</x:Column></x:ClientData></v:shape><v:shape id="textbox"><v:textbox><div>VML_SIGNAL</div></v:textbox></v:shape></xml>'
    return entries


def shape(text, kind="body", number=1):
    return f'<p:sp><p:nvSpPr><p:cNvPr id="{number}" name="Shape {number}"/><p:nvPr><p:ph type="{kind}"/></p:nvPr></p:nvSpPr><p:txBody><a:p><a:r><a:t>{xml(text)}</a:t></a:r></a:p></p:txBody></p:sp>'


def presentation(note="NOTE_SIGNAL TAB_01", body="BODY_SIGNAL TAB_010"):
    entries = base("ppt/presentation.xml", "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml")
    entries["ppt/presentation.xml"] = f'<p:presentation xmlns:p="{PPT}" xmlns:r="{R}"><p:sldIdLst><p:sldId id="256" r:id="slide"/></p:sldIdLst></p:presentation>'
    entries["ppt/_rels/presentation.xml.rels"] = rels([("slide", "slide", "slides/custom.xml", False)])
    entries["ppt/slides/custom.xml"] = f'<p:sld xmlns:p="{PPT}" xmlns:a="{A}"><p:cSld><p:spTree>{shape(body)}</p:spTree></p:cSld></p:sld>'
    entries["ppt/slides/_rels/custom.xml.rels"] = rels([("note", "notesSlide", "../extra/odd-notes.xml", False), ("again", "notesSlide", "../extra/odd-notes.xml", False)])
    entries["ppt/extra/odd-notes.xml"] = f'<p:notes xmlns:p="{PPT}" xmlns:a="{A}"><p:cSld><p:spTree>{shape(note)}' + ''.join(shape("IGNORED_SIGNAL", kind, i) for i, kind in enumerate(("dt", "ftr", "hdr", "sldNum", "sldImg"), 2)) + '</p:spTree></p:cSld></p:notes>'
    entries["ppt/extra/orphan.xml"] = f'<p:notes xmlns:p="{PPT}" xmlns:a="{A}">{shape("ORPHAN_SIGNAL")}</p:notes>'
    return entries


def word():
    entries = base("word/document.xml", "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml")
    paragraph = lambda text: f'<w:p><w:r><w:t>{xml(text)}</w:t></w:r></w:p>'
    section = '<w:sectPr><w:headerReference w:type="default" r:id="header"/><w:footerReference w:type="default" r:id="footer"/></w:sectPr>'
    entries["word/document.xml"] = f'<w:document xmlns:w="{W}" xmlns:r="{R}"><w:body>{paragraph("BODY_SIGNAL TAB_01")}{section}{section}</w:body></w:document>'
    entries["word/_rels/document.xml.rels"] = rels([("header", "header", "extra/shared-header.xml", False), ("footer", "footer", "extra/shared-footer.xml", False), ("comment", "comments", "extra/comment.xml", False)])
    entries["word/extra/shared-header.xml"] = f'<w:hdr xmlns:w="{W}">{paragraph("HEADER_SIGNAL")}<w:tbl><w:tr><w:tc>{paragraph("HEADER_TABLE_SIGNAL")}</w:tc></w:tr></w:tbl></w:hdr>'
    entries["word/extra/shared-footer.xml"] = f'<w:ftr xmlns:w="{W}">{paragraph("FOOTER_SIGNAL")}</w:ftr>'
    entries["word/extra/comment.xml"] = f'<w:comments xmlns:w="{W}"><w:comment w:id="99">{paragraph("WORD_COMMENT_SIGNAL")}</w:comment></w:comments>'
    return entries


def ranking_data(output):
    cases = []
    for index in range(1, 21):
        topic = f"RANK_{index:02d}"
        folder = output / "ranking" / f"case-{index:02d}"
        folder.mkdir(parents=True)
        case = {"name": f"rank-{index:02d}", "folder": str(folder.relative_to(output)), "query": topic, "extensions": "txt,xlsx,pptx", "flags": [], "prefer": "z-good.txt", "over": "a-other.txt"}
        category = (index-1) % 10
        if index == 11:  # The weakest AND match, not the first term's score, determines quality.
            case.update(query="", flags=["--fuzzy-search"], spec={"mode":"conditions","scope":"file","all":[topic,"customer"],"any":[],"not":[]})
            (folder / "z-good.txt").write_text(topic+"\ncustomer\n", encoding="utf-8")
            (folder / "a-other.txt").write_text(topic+"\ncustmer\n", encoding="utf-8")
        elif category == 0:  # Strong exact match beats many substring matches.
            (folder / "z-good.txt").write_text(topic, encoding="utf-8")
            (folder / "a-other.txt").write_text((f"prefix{topic}suffix\n" * 100), encoding="utf-8")
        elif category == 1:  # Same quality: body beats filename-only, even with a later path.
            case["over"] = f"{topic}.txt"
            (folder / "z-good.txt").write_text(f"x {topic}.txt y", encoding="utf-8")
            (folder / case["over"]).write_text("unrelated", encoding="utf-8")
        elif category in (2, 3):  # Row concentration / distinct OR coverage.
            other = f"OTHER_{index:02d}"
            case.update(query="", prefer="z-good.xlsx", over="a-other.xlsx", spec={"mode":"conditions", "scope":"file", "all": [topic, other] if category == 2 else [], "any": [] if category == 2 else [topic, other], "not": []})
            package(folder / "z-good.xlsx", workbook(f'<row r="1">{cell("A1",topic)}{cell("B1",other)}</row>'))
            rows = f'<row r="1">{cell("A1",topic)}</row>' + (f'<row r="2">{cell("A2",other)}</row>' if category == 2 else '')
            package(folder / "a-other.xlsx", workbook(rows))
        elif category == 4:  # Annotation-only vs body, with equal match quality.
            case.update(over="a-other.pptx", flags=["--include-notes"])
            (folder / "z-good.txt").write_text(topic, encoding="utf-8")
            package(folder / "a-other.pptx", presentation(topic,"unrelated"))
        elif category == 5:  # Capped repetition falls back to stable path order.
            case.update(prefer="a-good.txt", over="z-other.txt")
            (folder / "a-good.txt").write_text((topic+"\n")*5, encoding="utf-8")
            (folder / "z-other.txt").write_text((topic+"\n")*100, encoding="utf-8")
        elif category == 6:
            case.update(query="カタカナ"+str(index), flags=["--fuzzy-search"])
            (folder / "z-good.txt").write_text(case["query"], encoding="utf-8")
            (folder / "a-other.txt").write_text("かたかな"+str(index), encoding="utf-8")
        elif category == 7:
            case.update(query="customeridentifier"+("a"*index), flags=["--fuzzy-search"])
            (folder / "z-good.txt").write_text(case["query"], encoding="utf-8")
            (folder / "a-other.txt").write_text("customeridentifer"+("a"*index), encoding="utf-8")
        elif category == 8:
            case.update(query="CustomerKey"+str(index), flags=["--fuzzy-search"])
            (folder / "z-good.txt").write_text(case["query"], encoding="utf-8")
            (folder / "a-other.txt").write_text("customer_key_"+str(index), encoding="utf-8")
        else:  # Natural numeric cell ordering: A2 before A10.
            case.update(prefer="a-good.xlsx", over="z-other.xlsx", firstAddress="A2")
            rows = f'<row r="10">{cell("A10",topic)}</row><row r="2">{cell("A2",topic)}</row>'
            package(folder / "a-good.xlsx", workbook(rows))
            package(folder / "z-other.xlsx", workbook(rows))
        cases.append(case)
    (output / "ranking-cases.json").write_text(json.dumps(cases, ensure_ascii=False, indent=2)+"\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if output.exists(): parser.error(f"Output already exists: {output}")
    rows = f'<row r="1">{cell("A1","TAB_01")}{cell("B1","TAB_010")}<c r="C1"><f>IF(A1="FORMULA_SIGNAL",42,0)</f><v>42</v></c></row><row r="2"><c r="A2"><f t="shared" si="7" ref="A2:A3">SUM(A1,SHARED_SIGNAL)</f><v>7</v></c><c r="B2" t="e"><f>FORMULA_WITHOUT_CACHE</f><v>#VALUE!</v></c></row><row r="3"><c r="A3"><f t="shared" si="7"/><v>7</v></c></row>'
    package(output / "search" / "data.xlsx", workbook(rows, True))
    package(output / "macros" / "data.xlsm", workbook(rows, True, macro=True))
    package(output / "search" / "slides.pptx", presentation())
    package(output / "search" / "document.docx", word())
    for fault in ("malformed", "missing", "external", "invalid-utf8", "oversized"):
        entries = presentation()
        if fault == "missing": del entries["ppt/extra/odd-notes.xml"]
        elif fault == "external": entries["ppt/slides/_rels/custom.xml.rels"] = rels([("note","notesSlide","https://example.invalid/private.xml",True)])
        elif fault == "oversized": entries["ppt/extra/odd-notes.xml"] = "x"*(32*1024*1024+1)
        else: entries["ppt/extra/odd-notes.xml"] = {"malformed":"<broken", "invalid-utf8":b"\xff"}[fault]
        package(output / fault / "slides.pptx", entries)
    entries = word(); entries["word/extra/shared-header.xml"] = "<broken"
    package(output / "word-error" / "document.docx", entries)
    ranking_data(output)
    print(output)


if __name__ == "__main__": main()
