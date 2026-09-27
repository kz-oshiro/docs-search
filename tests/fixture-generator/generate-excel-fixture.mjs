import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { SpreadsheetFile, Workbook } from "@oai/artifact-tool";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const outputDir = path.resolve(scriptDir, "../fixtures");
const outputPath = path.join(outputDir, "doc-search-excel-test-fixture.xlsx");
const bodyFont = { name: "Arial", size: 10, color: "#243247" };

function styleHeader(sheet, address) {
  const range = sheet.getRange(address);
  range.format = {
    fill: "#1F4E78",
    font: { name: "Arial", size: 10, bold: true, color: "#FFFFFF" },
    horizontalAlignment: "center",
    verticalAlignment: "center",
    wrapText: true,
  };
}

function setWidths(sheet, widths) {
  for (const [address, width] of widths) {
    sheet.getRange(address).format.columnWidth = width;
  }
}

const workbook = Workbook.create();

const index = workbook.worksheets.add("ケース一覧");
const basic = workbook.worksheets.add("基本データ");
const types = workbook.worksheets.add("型チェック");
const formulas = workbook.worksheets.add("数式");
const layout = workbook.worksheets.add("レイアウト");

for (const sheet of [index, basic, types, formulas, layout]) {
  sheet.showGridLines = false;
}

index.getRange("A1:C5").values = [
  ["シート", "試験内容", "確認ポイント"],
  ["基本データ", "一般的な表の読み取り", "5行のデータ。IDの先頭ゼロ、0、空欄、日本語氏名を含む"],
  ["型チェック", "セルの値と型", "文字列・数値・真偽値・日付・日時・改行・数式風テキスト"],
  ["数式", "数式と計算結果", "小計の合計 2,460、税額 246、税込合計 2,706"],
  ["レイアウト", "結合セルと空行", "A1:C1が結合。5行目は空行で、6行目に続きのデータ"],
];
index.getRange("A1:C5").format.font = bodyFont;
styleHeader(index, "A1:C1");
index.getRange("A2:C5").format.verticalAlignment = "center";
index.getRange("C2:C5").format.wrapText = true;
setWidths(index, [["A1:A5", 18], ["B1:B5", 30], ["C1:C5", 55]]);
index.getRange("A1:C5").format.autofitRows();

basic.getRange("A2:A6").format.numberFormat = "@";
basic.getRange("A1:E6").values = [
  ["ID", "氏名", "部署", "件数", "金額（円）"],
  ["001", "佐藤 花子", "営業", 3, 12800],
  ["002", "李 明華", "開発", 0, 0],
  ["A-003", "山田 太郎", "営業", 12, 40500],
  ["004", "Zoë O’Neil", "企画", 2, 7600],
  ["005", "王 小明", "開発", null, 3250],
];
basic.getRange("A1:E6").format.font = bodyFont;
styleHeader(basic, "A1:E1");
basic.getRange("D2:E6").format.numberFormat = "#,##0";
setWidths(basic, [["A1:A6", 14], ["B1:B6", 22], ["C1:C6", 16], ["D1:D6", 12], ["E1:E6", 17]]);
basic.getRange("A1:E6").format.autofitRows();

types.getRange("B2").format.numberFormat = "@";
types.getRange("A1:C13").values = [
  ["項目", "値", "確認ポイント"],
  ["文字列（先頭ゼロ）", "00123", "文字列として保持"],
  ["数値", 123, "数値として保持"],
  ["ゼロ", 0, "空欄と区別"],
  ["空白", null, "値のないセル"],
  ["真偽値 true", true, "Boolean true"],
  ["真偽値 false", false, "Boolean false"],
  ["日付", new Date(Date.UTC(2026, 8, 27)), "日付シリアル値"],
  ["日時", new Date(Date.UTC(2026, 8, 27, 14, 35)), "日時シリアル値"],
  ["カンマ入り", "東京,大阪", "セル内のカンマ"],
  ["改行入り", "一行目\n二行目", "セル内改行"],
  ["数式風テキスト", "'=SUM(1,2)", "計算式ではなく文字列"],
  ["記号と日本語", "全角ＡＢＣ / 日本語 / 🙂 & < >", "Unicodeと記号"],
];
types.getRange("A1:C13").format.font = bodyFont;
styleHeader(types, "A1:C1");
types.getRange("B8").format.numberFormat = "yyyy-mm-dd";
types.getRange("B9").format.numberFormat = "yyyy-mm-dd hh:mm";
types.getRange("B11:B13").format.wrapText = true;
types.getRange("C2:C13").format.wrapText = true;
setWidths(types, [["A1:A13", 23], ["B1:B13", 36], ["C1:C13", 28]]);
types.getRange("A1:C13").format.autofitRows();

formulas.getRange("A1:D5").values = [
  ["品目", "単価（円）", "個数", "小計（円）"],
  ["りんご", 120, 3, null],
  ["コーヒー", 550, 2, null],
  ["ノート", 250, 4, null],
  ["合計", null, null, null],
];
formulas.getRange("D2:D4").formulas = [["=B2*C2"], ["=B3*C3"], ["=B4*C4"]];
formulas.getRange("D5").formulas = [["=SUM(D2:D4)"]];
formulas.getRange("A7:B9").values = [
  ["税率", 0.1],
  ["税額（円）", null],
  ["税込合計（円）", null],
];
formulas.getRange("B8:B9").formulas = [["=D5*B7"], ["=D5+B8"]];
formulas.getRange("A1:D9").format.font = bodyFont;
styleHeader(formulas, "A1:D1");
formulas.getRange("A5:D5").format.font = { name: "Arial", size: 10, bold: true, color: "#243247" };
formulas.getRange("B2:B5").format.numberFormat = "#,##0";
formulas.getRange("C2:C4").format.numberFormat = "#,##0";
formulas.getRange("D2:D5").format.numberFormat = "#,##0";
formulas.getRange("B7").format.numberFormat = "0%";
formulas.getRange("B8:B9").format.numberFormat = "#,##0";
setWidths(formulas, [["A1:A9", 22], ["B1:B9", 16], ["C1:C9", 12], ["D1:D9", 18]]);
formulas.getRange("A1:D9").format.autofitRows();

layout.mergeCells("A1:C1");
layout.getRange("A1").values = [["結合セルと空行の確認"]];
layout.getRange("A1:C1").format = {
  fill: "#D9EAF7",
  font: { name: "Arial", size: 14, bold: true, color: "#1F2937" },
  horizontalAlignment: "left",
  verticalAlignment: "center",
};
layout.getRange("A3:C3").values = [["ID", "分類", "メモ"]];
layout.getRange("A4:C4").values = [["L-001", "結合セル", "1行目はA1:C1を結合しています。"]];
layout.getRange("A6:C6").values = [["L-003", "空行の後", "5行目を空けて、6行目にデータがあります。"]];
layout.getRange("A1:C6").format.font = bodyFont;
layout.getRange("A1:C1").format = {
  fill: "#D9EAF7",
  font: { name: "Arial", size: 14, bold: true, color: "#1F2937" },
  horizontalAlignment: "left",
  verticalAlignment: "center",
};
styleHeader(layout, "A3:C3");
layout.getRange("C4:C6").format.wrapText = true;
setWidths(layout, [["A1:A6", 17], ["B1:B6", 20], ["C1:C6", 48]]);
layout.getRange("A1:C6").format.autofitRows();

workbook.recalculate();
await fs.mkdir(outputDir, { recursive: true });
const xlsx = await SpreadsheetFile.exportXlsx(workbook);
await xlsx.save(outputPath);
console.log(`OUTPUT ${outputPath}`);
