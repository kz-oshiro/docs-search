#!/usr/bin/env python3
"""Create small, separate Office/text inputs for P3 condition-search checks."""

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
    sheet_ns = common["SHEET"]

    def workbook(path, sheets):
        entries = {
            "[Content_Types].xml": (
                f'<Types xmlns="{content_types}"><Default Extension="rels" '
                'ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                '<Default Extension="xml" ContentType="application/xml"/>'
                '<Override PartName="/xl/workbook.xml" '
                'ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
                + ''.join(f'<Override PartName="/xl/worksheets/sheet{i}.xml" '
                          'ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
                          for i in range(1, len(sheets) + 1)) + '</Types>'
            ),
            "_rels/.rels": f'<Relationships xmlns="{package_rel}"><Relationship Id="rId1" Type="{document_rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>',
            "xl/workbook.xml": f'<workbook xmlns="{sheet_ns}" xmlns:r="{document_rel}"><sheets>'
                               + ''.join(f'<sheet name="{name}" sheetId="{i}" r:id="rId{i}"/>'
                                         for i, (name, _) in enumerate(sheets, 1)) + '</sheets></workbook>',
            "xl/_rels/workbook.xml.rels": f'<Relationships xmlns="{package_rel}">'
                                          + ''.join(f'<Relationship Id="rId{i}" Type="{document_rel}/worksheet" Target="worksheets/sheet{i}.xml"/>'
                                                    for i in range(1, len(sheets) + 1)) + '</Relationships>',
        }
        for i, (_, rows) in enumerate(sheets, 1):
            row_xml = ''.join(f'<row r="{number}">{"".join(cell(address, value) for address, value in values)}</row>'
                              for number, values in rows)
            entries[f"xl/worksheets/sheet{i}.xml"] = f'<worksheet xmlns="{sheet_ns}"><sheetData>{row_xml}</sheetData></worksheet>'
        package(path, entries)

    search = output / "search"
    workbook(search / "conditions.xlsx", [
        ("Items", [
            (12, [("A12", "顧客番号"), ("D12", "必須"), ("E12", "稼働")]),
            (13, [("A13", "顧客補足")]),
            (14, [("D14", "必須")]),
            (15, [("A15", "顧客番号"), ("D15", "必須"), ("E15", "廃止")]),
            (16, [("B16", "AND OR")]),
            (17, [("A17", "customer")]),
        ]),
        ("Other", [(12, [("A12", "顧客番号")])]),
    ])
    workbook(search / "filename-signal.xlsx", [
        ("Data", [(1, [("A1", "body-signal")])]),
    ])
    (search / "notes.txt").write_text("顧客\n必須\nAND OR\n", encoding="utf-8")
    errors = output / "errors"
    errors.mkdir()
    (errors / "filename-signal-broken.xlsx").write_bytes(b"not an OOXML archive")
    print(output)


if __name__ == "__main__":
    main()
