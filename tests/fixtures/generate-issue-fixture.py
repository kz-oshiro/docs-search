#!/usr/bin/env python3
"""Independent, deterministic data for issues #1-#9. No existing fixtures change."""
import argparse
from pathlib import Path
from zipfile import ZipFile, ZipInfo, ZIP_DEFLATED

NS = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'
REL = 'http://schemas.openxmlformats.org/package/2006/relationships'
OFFICE = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'


def workbook(path):
    parts = {
        '[Content_Types].xml': '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet10.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/></Types>',
        '_rels/.rels': f'<Relationships xmlns="{REL}"><Relationship Id="r1" Type="{OFFICE}/officeDocument" Target="xl/workbook.xml"/></Relationships>',
        'xl/workbook.xml': f'<workbook xmlns="{NS}" xmlns:r="{OFFICE}"><sheets><sheet name="Zeta" sheetId="10" r:id="z"/><sheet name="Alpha" sheetId="2" r:id="a"/></sheets></workbook>',
        'xl/_rels/workbook.xml.rels': f'<Relationships xmlns="{REL}"><Relationship Id="z" Type="{OFFICE}/worksheet" Target="worksheets/sheet10.xml"/><Relationship Id="a" Type="{OFFICE}/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="s" Type="{OFFICE}/sharedStrings" Target="sharedStrings.xml"/><Relationship Id="style" Type="{OFFICE}/styles" Target="styles.xml"/></Relationships>',
        'xl/sharedStrings.xml': f'<sst xmlns="{NS}" count="1" uniqueCount="1"><si><t>Visible 顧客 ORDER</t><rPh sb="8" eb="10"><t>PHONETIC_ONLY</t></rPh><phoneticPr fontId="0"/></si></sst>',
        'xl/worksheets/sheet10.xml': f'<worksheet xmlns="{NS}"><sheetFormatPr defaultRowHeight="15" defaultColWidth="12"/><cols><col min="1" max="2" width="20" customWidth="1"/><col min="26" max="27" width="14" customWidth="1" hidden="1"/></cols><sheetData><row r="10" ht="24" customHeight="1" hidden="1"><c r="A10" t="inlineStr"><is><t>ORDER ten</t></is></c></row><row r="2" ht="36" customHeight="1"><c r="AA2" s="1" t="inlineStr"><is><r><t>Rich visible </t></r><r><t>顧客 ORDER</t></r><rPh sb="13" eb="15"><t>RICH_PHONETIC_ONLY</t></rPh></is></c><c r="Z2" s="1" t="inlineStr"><is><t>Inline 顧客 ORDER</t><rPh sb="7" eb="9"><t>INLINE_PHONETIC_ONLY</t></rPh></is></c><c r="A2" s="1" t="s"><v>0</v></c><c r="B2" s="1"/></row></sheetData><mergeCells count="1"><mergeCell ref="A2:B2"/></mergeCells></worksheet>',
        'xl/worksheets/sheet2.xml': f'<worksheet xmlns="{NS}"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>ORDER alpha</t></is></c></row></sheetData></worksheet>',
        'xl/styles.xml': f'<styleSheet xmlns="{NS}"><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><i/><u/><sz val="14"/><color rgb="FF123456"/><name val="Arial"/></font></fonts><fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FFDDEEFF"/></patternFill></fill></fills><borders count="2"><border/><border><left style="thin"><color rgb="FF224466"/></left><right style="medium"><color rgb="FF224466"/></right><top style="dashed"><color rgb="FF224466"/></top><bottom style="double"><color rgb="FF224466"/></bottom></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="2" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center" wrapText="1"/></xf></cellXfs></styleSheet>',
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    with ZipFile(path, 'w') as archive:
        for name, text in sorted(parts.items()):
            entry = ZipInfo(name, date_time=(2020, 1, 1, 0, 0, 0))
            entry.compress_type = ZIP_DEFLATED
            archive.writestr(entry, text.encode('utf-8'))


def generate(output):
    workbook(output / 'excel' / 'layout.xlsx')
    for parent in ['left/same', 'right/same']:
        folder = output / parent
        folder.mkdir(parents=True, exist_ok=True)
        (folder / 'multiple.txt').write_bytes('対象 keep\r\n対象 skip\r\n対象 keep skip\r\n対象 KEEP\r\n'.encode('utf-8'))
        (folder / 'long.txt').write_bytes(('\ufeff' + 'あ' * 400 + '対象 対象 😀\r\n後文\r\n').encode('utf-8'))
        (folder / 'sjis.txt').write_bytes('対象 対象\r\nそのまま\r\n'.encode('cp932'))
    pages = output / 'pages'
    pages.mkdir(parents=True, exist_ok=True)
    for number in range(205):
        (pages / f'{number:03}.txt').write_bytes(f'対象 keep {number}\n'.encode('utf-8'))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('出力先には新しいディレクトリを指定してください。')
    generate(args.output)
