#!/usr/bin/env python3
"""Create one sparse Excel workbook for the P2 surrounding-cell checks."""

import argparse
import runpy
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if output.exists():
        parser.error(f"Output already exists: {output}")

    common = runpy.run_path(str(Path(__file__).with_name("generate-fixtures.py")))
    package = common["package"]
    cell = common["cell"]
    content_types = common["CONTENT_TYPES"]
    package_rel = common["PKG_REL"]
    document_rel = common["DOC_REL"]
    sheet = common["SHEET"]
    drawing = common["DRAWING"]
    xdr = common["XDR"]

    rows = [
        f'<row r="1">{cell("A1", "edge-top")}</row>',
        '<row r="11" hidden="1"/>',
        '<row r="12">' + ''.join([
            cell("A12", "左側"),
            cell("B12", "context-needle"),
            cell("C12", "結合メモ"),
            cell("E12", "cached-result", "formula"),
            cell("F12", "L" * 310),
            cell("Z12", "far-detail"),
        ]) + '</row>',
        f'<row r="1048576">{cell("XFD1048576", "edge-bottom")}</row>',
    ]
    sheet_xml = (
        f'<worksheet xmlns="{sheet}" xmlns:r="{document_rel}">'
        '<dimension ref="A1:XFD1048576"/>'
        '<cols><col min="4" max="4" hidden="1"/></cols>'
        f'<sheetData>{"".join(rows)}</sheetData>'
        '<mergeCells count="1"><mergeCell ref="C12:D12"/></mergeCells>'
        '<drawing r:id="rIdDrawing"/></worksheet>'
    )
    shape = lambda name, value, ident: (
        f'<xdr:sp><xdr:nvSpPr><xdr:cNvPr id="{ident}" name="{name}"/></xdr:nvSpPr>'
        f'<xdr:txBody><a:p><a:r><a:t>{value}</a:t></a:r></a:p></xdr:txBody></xdr:sp>'
    )
    drawing_xml = (
        f'<xdr:wsDr xmlns:xdr="{xdr}" xmlns:a="{drawing}">'
        '<xdr:oneCellAnchor><xdr:from><xdr:col>1</xdr:col><xdr:row>11</xdr:row></xdr:from>'
        f'{shape("Anchored", "context-shape", 1)}</xdr:oneCellAnchor>'
        f'<xdr:absoluteAnchor>{shape("Unanchored", "context-unanchored", 2)}</xdr:absoluteAnchor>'
        '</xdr:wsDr>'
    )
    entries = {
        "[Content_Types].xml": (
            f'<Types xmlns="{content_types}"><Default Extension="rels" '
            'ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
            '<Default Extension="xml" ContentType="application/xml"/>'
            '<Override PartName="/xl/workbook.xml" '
            'ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
            '<Override PartName="/xl/worksheets/sheet1.xml" '
            'ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
            '<Override PartName="/xl/drawings/drawing1.xml" '
            'ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/></Types>'
        ),
        "_rels/.rels": f'<Relationships xmlns="{package_rel}"><Relationship Id="rId1" Type="{document_rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>',
        "xl/workbook.xml": f'<workbook xmlns="{sheet}" xmlns:r="{document_rel}"><sheets><sheet name="Context" sheetId="1" r:id="rId1"/></sheets></workbook>',
        "xl/_rels/workbook.xml.rels": f'<Relationships xmlns="{package_rel}"><Relationship Id="rId1" Type="{document_rel}/worksheet" Target="worksheets/sheet1.xml"/></Relationships>',
        "xl/worksheets/sheet1.xml": sheet_xml,
        "xl/worksheets/_rels/sheet1.xml.rels": f'<Relationships xmlns="{package_rel}"><Relationship Id="rIdDrawing" Type="{document_rel}/drawing" Target="../drawings/drawing1.xml"/></Relationships>',
        "xl/drawings/drawing1.xml": drawing_xml,
    }
    package(output, entries)
    print(output)


if __name__ == "__main__":
    main()
