#!/usr/bin/env python3
"""Generate the stack-independent doc-search corpus with only the Python stdlib."""

import argparse
import json
import random
import shutil
from pathlib import Path
from xml.sax.saxutils import escape
from zipfile import ZIP_STORED, ZipFile, ZipInfo

SEED = 20260927
HERE = Path(__file__).resolve().parent
CONTENT_TYPES = "http://schemas.openxmlformats.org/package/2006/content-types"
PKG_REL = "http://schemas.openxmlformats.org/package/2006/relationships"
DOC_REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
SHEET = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
DRAWING = "http://schemas.openxmlformats.org/drawingml/2006/main"
XDR = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"
PRESENTATION = "http://schemas.openxmlformats.org/presentationml/2006/main"
WORD = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"


def xml(text):
    return escape(str(text), {'"': "&quot;", "'": "&apos;"})


def package(path, entries):
    path.parent.mkdir(parents=True, exist_ok=True)
    with ZipFile(path, "w") as archive:
        for name, content in entries.items():
            info = ZipInfo(name, date_time=(2020, 1, 1, 0, 0, 0))
            info.compress_type = ZIP_STORED
            info.create_system = 0
            info.external_attr = 0
            archive.writestr(info, content.encode("utf-8"))


def column(number):
    result = ""
    while number:
        number, remainder = divmod(number - 1, 26)
        result = chr(65 + remainder) + result
    return result


def cell(address, value, kind="inlineStr"):
    if kind == "inlineStr":
        return f'<c r="{address}" t="inlineStr"><is><t>{xml(value)}</t></is></c>'
    if kind == "formula":
        return f'<c r="{address}" t="str"><f>"needle"</f><v>{xml(value)}</v></c>'
    return f'<c r="{address}"' + (f' t="{kind}"' if kind != "n" else "") + f'><v>{xml(value)}</v></c>'


def worksheet(number, role, rng, rows=80, cols=12):
    first = []
    if role == "needle" and number == 1:
        first = [cell("A1", 0, "s"), cell("B1", "Inline needle"), cell("C1", 42, "n"), cell("D1", "needle", "formula"), cell("E1", 1, "b")]
    elif role == "needle" and number == 2:
        first = [cell("A1", "hidden-beacon")]
    elif role == "macro" and number == 1:
        first = [cell("A1", "BEACON macro value")]
    else:
        first = [cell("A1", f"Document sheet {number} inventory")]
    first.extend(cell(f"{column(c)}1", f"field {c:02d}") for c in range(len(first) + 1, cols + 1))
    lines = [f'<row r="1">{"".join(first)}</row>']
    if role == "needle" and number == 1:
        lines.append('<row r="2"><c r="A2" t="s"><v>1</v></c>' + ''.join(cell(f"{column(c)}2", rng.randrange(100000, 999999), "n") for c in range(2, cols + 1)) + '</row>')
        start = 3
    else:
        start = 2
    for row in range(start, rows + 1):
        values = [cell(f"{column(c)}{row}", f"record {number:02d} {row:03d} {c:02d} reference {rng.randrange(100000, 999999)}") if c in (1, 4, 8, 12) else cell(f"{column(c)}{row}", rng.randrange(100000, 999999), "n") for c in range(1, cols + 1)]
        lines.append(f'<row r="{row}">{"".join(values)}</row>')
    drawing = '<drawing xmlns:r="' + DOC_REL + '" r:id="rIdDrawing"/><legacyDrawing xmlns:r="' + DOC_REL + '" r:id="rIdVml"/>' if role == "needle" and number == 1 else ""
    return f'<worksheet xmlns="{SHEET}"><dimension ref="A1:{column(cols)}{rows}"/><sheetData>{"".join(lines)}</sheetData>{drawing}</worksheet>'


def workbook(path, role, seed, sheets=3, rows=80, cols=12):
    rng = random.Random(f"{seed}:{role}:{path.name}")
    macro = path.suffix == ".xlsm"
    main_type = "application/vnd.ms-excel.sheet.macroEnabled.main+xml" if macro else "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
    overrides = ''.join(f'<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>' for i in range(1, sheets + 1))
    if role == "needle":
        overrides += '<Override PartName="/xl/drawings/drawing1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/><Default Extension="vml" ContentType="application/vnd.openxmlformats-officedocument.vmlDrawing"/>'
    sheet_entries = []
    for i in range(1, sheets + 1):
        name = ["Main", "Hidden", "Data"][i - 1] if i <= 3 else f"Data{i:02d}"
        state = ' state="hidden"' if role == "needle" and i == 2 else ""
        sheet_entries.append(f'<sheet name="{name}" sheetId="{i}" r:id="rId{i}"{state}/>')
    entries = {
        "[Content_Types].xml": f'<Types xmlns="{CONTENT_TYPES}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="{main_type}"/>{overrides}</Types>',
        "_rels/.rels": f'<Relationships xmlns="{PKG_REL}"><Relationship Id="rId1" Type="{DOC_REL}/officeDocument" Target="xl/workbook.xml"/></Relationships>',
        "xl/workbook.xml": f'<workbook xmlns="{SHEET}" xmlns:r="{DOC_REL}"><sheets>' + ''.join(sheet_entries) + '</sheets></workbook>',
        "xl/_rels/workbook.xml.rels": f'<Relationships xmlns="{PKG_REL}">' + ''.join(f'<Relationship Id="rId{i}" Type="{DOC_REL}/worksheet" Target="worksheets/sheet{i}.xml"/>' for i in range(1, sheets + 1)) + (f'<Relationship Id="rIdShared" Type="{DOC_REL}/sharedStrings" Target="sharedStrings.xml"/>' if role == "needle" else '') + '</Relationships>',
    }
    if role == "needle":
        entries["xl/sharedStrings.xml"] = f'<sst xmlns="{SHEET}" count="2" uniqueCount="2"><si><t>Alpha needle</t></si><si><r><t>Rich </t></r><r><t>needle</t></r></si></sst>'
        entries["xl/worksheets/_rels/sheet1.xml.rels"] = f'<Relationships xmlns="{PKG_REL}"><Relationship Id="rIdDrawing" Type="{DOC_REL}/drawing" Target="../drawings/drawing1.xml"/><Relationship Id="rIdVml" Type="{DOC_REL}/vmlDrawing" Target="../drawings/vmlDrawing1.vml"/></Relationships>'
        entries["xl/drawings/drawing1.xml"] = f'<xdr:wsDr xmlns:xdr="{XDR}" xmlns:a="{DRAWING}"><xdr:twoCellAnchor><xdr:from><xdr:col>2</xdr:col><xdr:row>3</xdr:row></xdr:from><xdr:grpSp><xdr:sp><xdr:nvSpPr><xdr:cNvPr id="1" name="Callout"/></xdr:nvSpPr><xdr:txBody><a:p><a:r><a:t>Shape </a:t></a:r><a:r><a:t>needle</a:t></a:r></a:p></xdr:txBody></xdr:sp></xdr:grpSp></xdr:twoCellAnchor></xdr:wsDr>'
        entries["xl/drawings/vmlDrawing1.vml"] = '<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:x="urn:schemas-microsoft-com:office:excel"><v:shape id="LegacyBox"><v:textbox><div>VML needle</div></v:textbox><x:ClientData><x:Row>4</x:Row><x:Column>1</x:Column></x:ClientData></v:shape></xml>'
    for i in range(1, sheets + 1):
        entries[f"xl/worksheets/sheet{i}.xml"] = worksheet(i, role, rng, rows, cols)
    package(path, entries)


def shape(name, text, ident=2, split=False):
    runs = [text] if not split else [text[:3], text[3:]]
    paragraph = ''.join(f'<a:r><a:t>{xml(part)}</a:t></a:r>' for part in runs)
    return f'<p:sp><p:nvSpPr><p:cNvPr id="{ident}" name="{xml(name)}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p>{paragraph}</a:p></p:txBody></p:sp>'


def presentation(path, seed, slides=8, shapes_per_slide=6):
    rng = random.Random(f"{seed}:{path.name}")
    slide_ids = ''.join(f'<p:sldId id="{255+i}" r:id="rId{i}"/>' for i in range(1, slides + 1))
    group = '<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>'
    colors = ''.join(f'<a:{name}><a:srgbClr val="{value}"/></a:{name}>' for name, value in (("dk1", "000000"), ("lt1", "FFFFFF"), ("dk2", "202020"), ("lt2", "EEEEEE"), ("accent1", "4472C4"), ("accent2", "ED7D31"), ("accent3", "A5A5A5"), ("accent4", "FFC000"), ("accent5", "5B9BD5"), ("accent6", "70AD47"), ("hlink", "0563C1"), ("folHlink", "954F72")))
    theme = f'<a:theme xmlns:a="{DRAWING}" name="Test theme"><a:themeElements><a:clrScheme name="Test">{colors}</a:clrScheme><a:fontScheme name="Test"><a:majorFont><a:latin typeface="Arial"/></a:majorFont><a:minorFont><a:latin typeface="Arial"/></a:minorFont></a:fontScheme><a:fmtScheme name="Test"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln/></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>'
    entries = {
        "[Content_Types].xml": f'<Types xmlns="{CONTENT_TYPES}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/><Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/><Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>' + ''.join(f'<Override PartName="/ppt/slides/slide{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>' for i in range(1, slides + 1)) + '</Types>',
        "_rels/.rels": f'<Relationships xmlns="{PKG_REL}"><Relationship Id="rId1" Type="{DOC_REL}/officeDocument" Target="ppt/presentation.xml"/></Relationships>',
        "ppt/presentation.xml": f'<p:presentation xmlns:p="{PRESENTATION}" xmlns:r="{DOC_REL}"><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rIdMaster"/></p:sldMasterIdLst><p:sldIdLst>{slide_ids}</p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>',
        "ppt/_rels/presentation.xml.rels": f'<Relationships xmlns="{PKG_REL}"><Relationship Id="rIdMaster" Type="{DOC_REL}/slideMaster" Target="slideMasters/slideMaster1.xml"/>' + ''.join(f'<Relationship Id="rId{i}" Type="{DOC_REL}/slide" Target="slides/slide{i}.xml"/>' for i in range(1, slides + 1)) + '</Relationships>',
        "ppt/slideMasters/slideMaster1.xml": f'<p:sldMaster xmlns:p="{PRESENTATION}" xmlns:a="{DRAWING}" xmlns:r="{DOC_REL}"><p:cSld><p:spTree>{group}</p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/><p:sldLayoutIdLst><p:sldLayoutId id="1" r:id="rIdLayout"/></p:sldLayoutIdLst></p:sldMaster>',
        "ppt/slideMasters/_rels/slideMaster1.xml.rels": f'<Relationships xmlns="{PKG_REL}"><Relationship Id="rIdLayout" Type="{DOC_REL}/slideLayout" Target="../slideLayouts/slideLayout1.xml"/><Relationship Id="rIdTheme" Type="{DOC_REL}/theme" Target="../theme/theme1.xml"/></Relationships>',
        "ppt/slideLayouts/slideLayout1.xml": f'<p:sldLayout xmlns:p="{PRESENTATION}" xmlns:a="{DRAWING}" type="blank"><p:cSld><p:spTree>{group}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>',
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels": f'<Relationships xmlns="{PKG_REL}"><Relationship Id="rIdMaster" Type="{DOC_REL}/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>',
        "ppt/theme/theme1.xml": theme,
    }
    for slide in range(1, slides + 1):
        items = []
        for position in range(1, shapes_per_slide + 1):
            special = {1: "Beacon presentation title", 2: "BEACON split phrase", 4: "Beacon hidden slide"}.get(slide) if position == 1 else None
            text = special or f"Slide {slide:02d} topic {position:02d} evidence {rng.randrange(100000, 999999)} for regional planning"
            items.append(shape(f"Text {position}", text, position + 1, split=(slide == 2 and position == 1)))
        if slide == 3:
            items.append(f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="90" name="Evidence table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblGrid><a:gridCol w="5000000"/></a:tblGrid><a:tr h="500000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Beacon table entry</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>')
        entries[f"ppt/slides/slide{slide}.xml"] = f'<p:sld xmlns:p="{PRESENTATION}" xmlns:a="{DRAWING}"' + (' show="0"' if slide == 4 else '') + f'><p:cSld><p:spTree>{group}{"".join(items)}</p:spTree></p:cSld></p:sld>'
        entries[f"ppt/slides/_rels/slide{slide}.xml.rels"] = f'<Relationships xmlns="{PKG_REL}"><Relationship Id="rIdLayout" Type="{DOC_REL}/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>'
    package(path, entries)


def paragraph(text, split=False):
    pieces = [text] if not split else [text[:3], text[3:]]
    return '<w:p>' + ''.join(f'<w:r><w:t>{xml(piece)}</w:t></w:r>' for piece in pieces) + '</w:p>'


def word_document(path, seed, paragraphs=50):
    rng = random.Random(f"{seed}:{path.name}")
    body = [paragraph("Beacon field guide"), paragraph("BEACON followup", True)]
    body.extend(paragraph(f"Section {i:03d}: regional operations record {rng.randrange(100000, 999999)}. This paragraph carries stable sample content for search and extraction.") for i in range(3, paragraphs + 1))
    table_start = '<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w="5000"/></w:tblGrid><w:tr><w:tc><w:tcPr/>'
    nested = table_start + paragraph("Beacon nested table") + '</w:tc></w:tr></w:tbl>'
    body.append(table_start + paragraph("Beacon table entry") + nested + paragraph("End of table cell") + '</w:tc></w:tr></w:tbl>')
    entries = {
        "[Content_Types].xml": f'<Types xmlns="{CONTENT_TYPES}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>',
        "_rels/.rels": f'<Relationships xmlns="{PKG_REL}"><Relationship Id="rId1" Type="{DOC_REL}/officeDocument" Target="word/document.xml"/></Relationships>',
        "word/document.xml": f'<w:document xmlns:w="{WORD}"><w:body>{"".join(body)}<w:sectPr/></w:body></w:document>',
    }
    package(path, entries)


def text_document(path, seed, lines=150, bom=False):
    rng = random.Random(f"{seed}:{path.name}")
    content = [f"Record {i:03d}: project status and notes {rng.randrange(100000, 999999)} with enough words to exercise long line previews." for i in range(1, lines + 1)]
    content[0] = "BOM beacon" if bom else "Beacon alpha"
    if not bom:
        content[24] = "beacon beta"
        content[69] = "literal a+b expression"
        content[70] = "decomposed cafe\u0301 and Straße"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(("\ufeff" if bom else "").encode("utf-8") + ("\n".join(content) + "\n").encode("utf-8"))


def code_document(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def generate(output, profile, seed):
    output = output.resolve()
    if output.exists() and any(output.iterdir()):
        raise ValueError(f"Output directory must be empty: {output}")
    root = output / "search"
    workbook(root / "spreadsheets" / "needle-book.xlsx", "needle", seed)
    workbook(root / "spreadsheets" / "filename-token.xlsx", "plain", seed)
    workbook(root / "nested" / "quarterly-ledger.xlsm", "macro", seed)
    presentation(root / "nested" / "team-briefing.pptx", seed)
    word_document(root / "documents" / "operations-guide.docx", seed)
    text_document(root / "notes" / "research-log.txt", seed)
    text_document(root / "notes" / "bom-note.txt", seed, bom=True)
    code_document(root / "code" / "sample.jsp", '<%@ page contentType="text/html" %>\n<p>Beacon JSP sample</p>\n')
    code_document(root / "code" / "sample.xhtml", '<?xml version="1.0"?>\n<html><body>Beacon XHTML sample</body></html>\n')
    code_document(root / "code" / "sample.html", '<!doctype html>\n<p>Beacon HTML sample</p>\n')
    code_document(root / "code" / "sample.js", 'const label = "Beacon JS sample";\n')
    code_document(root / "code" / "sample.java", 'class Sample { String label = "Beacon Java sample"; }\n')
    (root / "spreadsheets" / "broken.xlsx").write_bytes(b"not an OOXML archive")
    (root / "notes" / "invalid-utf8.txt").write_bytes(b"invalid utf8: \xff\xfe\n")
    (root / "spreadsheets" / "~$temporary.xlsx").write_bytes(b"excluded office temporary file")
    (root / "notes" / "legacy.xls").write_bytes(b"unsupported format beacon")
    if profile == "load":
        for i in range(1, 21):
            workbook(output / "load" / f"book-{i:03d}.xlsx", "load", seed, sheets=5, rows=250, cols=16)
            presentation(output / "load" / f"slides-{i:03d}.pptx", seed, slides=20, shapes_per_slide=8)
            word_document(output / "load" / f"report-{i:03d}.docx", seed, paragraphs=150)
            text_document(output / "load" / f"notes-{i:03d}.txt", seed, lines=1000)
    shutil.copyfile(HERE / "backend-cases.json", output / "backend-cases.json")
    manifest = {"schemaVersion": 1, "profile": profile, "seed": seed, "files": sorted(str(p.relative_to(output)).replace("\\", "/") for p in output.rglob("*") if p.is_file() and p.name != "backend-cases.json")}
    (output / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Generated {len(manifest['files'])} files in {output}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=HERE / "generated", help="new or empty output directory (default: tests/generated)")
    parser.add_argument("--profile", choices=("acceptance", "load"), default="acceptance")
    args = parser.parse_args()
    generate(args.output, args.profile, SEED)


if __name__ == "__main__":
    main()
