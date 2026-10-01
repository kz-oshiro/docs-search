use encoding_rs::SHIFT_JIS;
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;
use zip::ZipArchive;

const MAX_PART: u64 = 32 * 1024 * 1024;
const MAX_TOTAL: u64 = 128 * 1024 * 1024;
const MAX_PARTS: usize = 4096;

#[derive(Clone, Debug)]
pub struct Unit {
    pub source_kind: &'static str,
    pub location: Value,
    pub text: String,
    pub meta: UnitMeta,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitMeta {
    pub unit_key: String,
    pub part_key: String,
    pub group_key: String,
    pub row: Option<u32>,
    pub column: Option<u32>,
    pub content_class: String,
    pub anchor: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetMeta {
    pub part_key: String,
    pub sheet_name: String,
    pub merge_ranges: Vec<CellRange>,
    pub hidden_rows: Vec<u32>,
    pub hidden_columns: Vec<ColumnRange>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CellRange {
    pub first_row: u32,
    pub first_column: u32,
    pub last_row: u32,
    pub last_column: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnRange {
    pub first: u32,
    pub last: u32,
}

#[derive(Debug, Default)]
pub struct ExtractedDocument {
    pub units: Vec<Unit>,
    pub sheets: Vec<SheetMeta>,
}

impl Unit {
    pub fn new(source_kind: &'static str, location: Value, text: String) -> Self {
        Self {
            source_kind,
            location,
            text,
            meta: UnitMeta {
                content_class: "body".into(),
                ..UnitMeta::default()
            },
        }
    }

    fn in_part(mut self, part: &str) -> Self {
        self.meta.part_key = part.to_owned();
        self
    }

    fn excel_cell(part: &str, sheet: &str, address: &str, text: String) -> Self {
        let mut unit = Self::new(
            "cell",
            json!({"sheetName": sheet, "cellAddress": address}),
            text,
        )
        .in_part(part);
        if let Some((row, column)) = parse_cell_address(address) {
            unit.meta.row = Some(row);
            unit.meta.column = Some(column);
            unit.meta.group_key = format!("{part}#row:{row}");
        }
        unit
    }
}

fn assign_keys(units: &mut [Unit]) {
    for (index, unit) in units.iter_mut().enumerate() {
        unit.meta.unit_key = (index + 1).to_string();
        if unit.meta.group_key.is_empty() {
            unit.meta.group_key = format!("unit:{}", index + 1);
        }
    }
}

pub(crate) fn parse_cell_address(address: &str) -> Option<(u32, u32)> {
    let mut column = 0u32;
    let mut boundary = 0;
    for (index, ch) in address.char_indices() {
        if !ch.is_ascii_alphabetic() {
            break;
        }
        column = column
            .checked_mul(26)?
            .checked_add(u32::from(ch.to_ascii_uppercase() as u8 - b'A') + 1)?;
        boundary = index + ch.len_utf8();
    }
    if boundary == 0 || column == 0 || column > 16_384 {
        return None;
    }
    let row = address[boundary..].parse::<u32>().ok()?;
    if row == 0 || row > 1_048_576 {
        return None;
    }
    Some((row, column))
}

fn sheet_meta(doc: &Document<'_>, part: &str, name: &str) -> SheetMeta {
    let mut sheet = SheetMeta {
        part_key: part.to_owned(),
        sheet_name: name.to_owned(),
        ..SheetMeta::default()
    };
    for merge in doc.descendants().filter(|node| element(*node, "mergeCell")) {
        let Some((first, last)) = attr(merge, "ref").and_then(|value| value.split_once(':')) else {
            continue;
        };
        if let (Some((first_row, first_column)), Some((last_row, last_column))) =
            (parse_cell_address(first), parse_cell_address(last))
        {
            if first_row <= last_row && first_column <= last_column {
                sheet.merge_ranges.push(CellRange {
                    first_row,
                    first_column,
                    last_row,
                    last_column,
                });
            }
        }
    }
    for row in doc.descendants().filter(|node| element(*node, "row")) {
        if matches!(attr(row, "hidden"), Some("1" | "true")) {
            if let Some(number) = attr(row, "r").and_then(|value| value.parse::<u32>().ok()) {
                if (1..=1_048_576).contains(&number) {
                    sheet.hidden_rows.push(number);
                }
            }
        }
    }
    for column in doc.descendants().filter(|node| element(*node, "col")) {
        if matches!(attr(column, "hidden"), Some("1" | "true")) {
            if let (Some(first), Some(last)) = (
                attr(column, "min").and_then(|value| value.parse::<u32>().ok()),
                attr(column, "max").and_then(|value| value.parse::<u32>().ok()),
            ) {
                if first > 0 && first <= last && last <= 16_384 {
                    sheet.hidden_columns.push(ColumnRange { first, last });
                }
            }
        }
    }
    sheet
}

#[derive(Debug)]
pub struct ExtractError {
    pub code: &'static str,
    pub message: String,
}

impl ExtractError {
    fn unreadable(message: impl Into<String>) -> Self {
        Self {
            code: "unreadable",
            message: message.into(),
        }
    }
    pub fn limit(message: impl Into<String>) -> Self {
        Self {
            code: "resourceLimit",
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for ExtractError {
    fn from(error: std::io::Error) -> Self {
        let code = match error.kind() {
            std::io::ErrorKind::PermissionDenied => "accessDenied",
            std::io::ErrorKind::NotFound => "changedDuringRead",
            _ => "unreadable",
        };
        Self {
            code,
            message: error.to_string(),
        }
    }
}

fn element<'a, 'input>(node: Node<'a, 'input>, name: &str) -> bool {
    node.is_element() && node.tag_name().name() == name
}
fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children().find(|n| element(*n, name))
}
fn attr<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<&'a str> {
    if name == "id" {
        if let Some(value) = node.attribute((
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
            "id",
        )) {
            return Some(value);
        }
    }
    node.attributes()
        .find(|a| a.name() == name)
        .map(|a| a.value())
}
fn all_text(node: Node<'_, '_>, tag: &str) -> String {
    node.descendants()
        .filter(|n| element(*n, tag))
        .filter_map(|n| n.text())
        .collect()
}
fn parsed<'a>(
    archive: &'a HashMap<String, String>,
    part: &str,
) -> Result<Document<'a>, ExtractError> {
    let xml = archive
        .get(part)
        .ok_or_else(|| ExtractError::unreadable(format!("必要な部品がありません: {part}")))?;
    Document::parse(xml)
        .map_err(|_| ExtractError::unreadable(format!("XML を解析できません: {part}")))
}

fn normalize_part(base: &str, target: &str) -> Result<String, ExtractError> {
    if target.contains(':') || target.starts_with("//") || target.contains('\\') {
        return Err(ExtractError::unreadable("外部または不正な参照先です。"));
    }
    let mut parts: Vec<&str> = if target.starts_with('/') {
        vec![]
    } else {
        base.rsplit_once('/')
            .map(|(d, _)| d.split('/').collect())
            .unwrap_or_default()
    };
    for part in target.trim_start_matches('/').split('/') {
        match part {
            "" | "." => (),
            ".." => {
                if parts.pop().is_none() {
                    return Err(ExtractError::unreadable("参照先がパッケージ外です。"));
                }
            }
            _ => parts.push(part),
        }
    }
    Ok(parts.join("/"))
}

fn rels_part(source: &str) -> String {
    match source.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{source}.rels"),
    }
}

fn relationships(
    archive: &HashMap<String, String>,
    source: &str,
) -> Result<HashMap<String, (String, String)>, ExtractError> {
    let part = rels_part(source);
    if !archive.contains_key(&part) {
        return Ok(HashMap::new());
    }
    let doc = parsed(archive, &part)?;
    let mut map = HashMap::new();
    for rel in doc.descendants().filter(|n| element(*n, "Relationship")) {
        if attr(rel, "TargetMode") == Some("External") {
            continue;
        }
        if let (Some(id), Some(target), Some(kind)) =
            (attr(rel, "Id"), attr(rel, "Target"), attr(rel, "Type"))
        {
            map.insert(
                id.to_owned(),
                (normalize_part(source, target)?, kind.to_owned()),
            );
        }
    }
    Ok(map)
}

fn main_part(archive: &HashMap<String, String>, suffix: &str) -> Result<String, ExtractError> {
    let doc = parsed(archive, "_rels/.rels")?;
    for rel in doc.descendants().filter(|n| element(*n, "Relationship")) {
        if attr(rel, "Type").is_some_and(|s| s.ends_with("/officeDocument"))
            && attr(rel, "TargetMode") != Some("External")
        {
            let path = normalize_part("", attr(rel, "Target").unwrap_or(""))?;
            if path.ends_with(suffix) {
                return Ok(path);
            }
        }
    }
    Err(ExtractError::unreadable("主文書部品がありません。"))
}

fn read_archive<R: Read + Seek>(reader: R) -> Result<HashMap<String, String>, ExtractError> {
    let mut zip = ZipArchive::new(reader)
        .map_err(|_| ExtractError::unreadable("Office ファイルを ZIP として開けません。"))?;
    if zip.len() > MAX_PARTS {
        return Err(ExtractError::limit("ZIP の部品数が上限を超えました。"));
    }
    let mut total = 0u64;
    let mut parts = HashMap::new();
    for index in 0..zip.len() {
        let mut item = zip.by_index(index).map_err(|error| {
            let message = error.to_string();
            if message.to_ascii_lowercase().contains("password")
                || message.to_ascii_lowercase().contains("encrypt")
            {
                ExtractError {
                    code: "encrypted",
                    message: "暗号化された Office ファイルは読めません。".into(),
                }
            } else {
                ExtractError::unreadable("ZIP の部品を開けません。")
            }
        })?;
        let name = item.name().to_owned();
        if !(name.ends_with(".xml") || name.ends_with(".rels") || name.ends_with(".vml")) {
            continue;
        }
        if name.starts_with('/') || name.contains("../") || name.contains('\\') {
            return Err(ExtractError::unreadable("ZIP の部品名が不正です。"));
        }
        if item.size() > MAX_PART || total.saturating_add(item.size()) > MAX_TOTAL {
            return Err(ExtractError::limit(
                "Office ファイルの展開量が上限を超えました。",
            ));
        }
        let mut bytes = Vec::with_capacity(item.size() as usize);
        item.by_ref().take(MAX_PART + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_PART {
            return Err(ExtractError::limit("Office 部品が上限を超えました。"));
        }
        total += bytes.len() as u64;
        let content = String::from_utf8(bytes)
            .map_err(|_| ExtractError::unreadable("Office XML が UTF-8 ではありません。"))?;
        if parts.insert(name, content).is_some() {
            return Err(ExtractError::unreadable("ZIP に重複した部品があります。"));
        }
    }
    Ok(parts)
}

pub fn text_units(bytes: &[u8]) -> Result<Vec<Unit>, ExtractError> {
    let text = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        // A UTF-8 BOM identifies the encoding even when the remaining bytes are damaged.
        std::str::from_utf8(&bytes[3..])
            .map(std::borrow::Cow::Borrowed)
            .map_err(|_| ExtractError {
                code: "unsupportedEncoding",
                message: "UTF-8 BOM のあるファイルを UTF-8 として読めません。".into(),
            })?
    } else if let Ok(utf8) = std::str::from_utf8(bytes) {
        std::borrow::Cow::Borrowed(utf8)
    } else {
        SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(bytes)
            .ok_or_else(|| ExtractError {
                code: "unsupportedEncoding",
                message: "UTF-8 または Shift_JIS として読めません。".into(),
            })?
    };
    let mut units: Vec<Unit> = text
        .lines()
        .enumerate()
        .map(|(i, line)| {
            Unit::new(
                "textLine",
                json!({"lineNumber": i + 1}),
                line.trim_end_matches('\r').to_owned(),
            )
        })
        .collect();
    assign_keys(&mut units);
    Ok(units)
}

pub fn office_units(path: &Path, file_type: &str) -> Result<ExtractedDocument, ExtractError> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_TOTAL {
        return Err(ExtractError::limit(
            "Office ファイルがサイズ上限を超えました。",
        ));
    }
    let archive = read_archive(File::open(path)?)?;
    let mut document = match file_type {
        "xlsx" | "xlsm" => excel_units(&archive),
        "pptx" => powerpoint_units(&archive).map(|units| ExtractedDocument {
            units,
            sheets: vec![],
        }),
        "docx" => word_units(&archive).map(|units| ExtractedDocument {
            units,
            sheets: vec![],
        }),
        _ => Err(ExtractError::unreadable("対応していない形式です。")),
    }?;
    assign_keys(&mut document.units);
    Ok(document)
}

fn excel_units(archive: &HashMap<String, String>) -> Result<ExtractedDocument, ExtractError> {
    let workbook_part = main_part(archive, "workbook.xml")?;
    let workbook = parsed(archive, &workbook_part)?;
    let rels = relationships(archive, &workbook_part)?;
    let mut strings = Vec::new();
    if let Some((part, _)) = rels
        .values()
        .find(|(_, kind)| kind.ends_with("/sharedStrings"))
    {
        let doc = parsed(archive, part)?;
        for si in doc.descendants().filter(|n| element(*n, "si")) {
            strings.push(all_text(si, "t"));
        }
    }
    let mut units = Vec::new();
    let mut sheets = Vec::new();
    for sheet in workbook.descendants().filter(|n| element(*n, "sheet")) {
        let name = attr(sheet, "name").unwrap_or("(名称なし)");
        let id = attr(sheet, "id")
            .ok_or_else(|| ExtractError::unreadable("シートの参照 ID がありません。"))?;
        let part = rels
            .get(id)
            .filter(|(_, kind)| kind.ends_with("/worksheet"))
            .map(|(part, _)| part)
            .ok_or_else(|| ExtractError::unreadable("シートの参照先がありません。"))?;
        let doc = parsed(archive, part)?;
        sheets.push(sheet_meta(&doc, part, name));
        for cell in doc.descendants().filter(|n| element(*n, "c")) {
            let address = attr(cell, "r").unwrap_or("");
            if address.is_empty() {
                continue;
            }
            let kind = attr(cell, "t").unwrap_or("n");
            let value = if kind == "inlineStr" {
                child(cell, "is")
                    .map(|n| all_text(n, "t"))
                    .unwrap_or_default()
            } else {
                child(cell, "v")
                    .and_then(|n| n.text())
                    .unwrap_or("")
                    .to_owned()
            };
            if value.is_empty() {
                continue;
            }
            let value =
                match kind {
                    "s" => strings
                        .get(value.parse::<usize>().map_err(|_| {
                            ExtractError::unreadable("共有文字列の番号が不正です。")
                        })?)
                        .ok_or_else(|| ExtractError::unreadable("共有文字列がありません。"))?
                        .clone(),
                    "b" => {
                        if value == "1" {
                            "TRUE".into()
                        } else {
                            "FALSE".into()
                        }
                    }
                    "e" => continue,
                    _ => value,
                };
            units.push(Unit::excel_cell(part, name, address, value));
        }
        let sheet_rels = relationships(archive, part)?;
        for link in doc
            .descendants()
            .filter(|n| element(*n, "drawing") || element(*n, "legacyDrawing"))
        {
            let Some(id) = attr(link, "id") else { continue };
            let Some((drawing_part, _)) = sheet_rels.get(id) else {
                continue;
            };
            let drawing = parsed(archive, drawing_part)?;
            if element(link, "legacyDrawing") {
                excel_vml(&drawing, name, drawing_part, &mut units);
            } else {
                excel_drawing(&drawing, name, drawing_part, &mut units);
            }
        }
    }
    Ok(ExtractedDocument { units, sheets })
}

fn excel_drawing(doc: &Document<'_>, sheet: &str, part: &str, units: &mut Vec<Unit>) {
    for anchor in doc.descendants().filter(|n| {
        n.is_element()
            && ["twoCellAnchor", "oneCellAnchor", "absoluteAnchor"].contains(&n.tag_name().name())
    }) {
        let position = child(anchor, "from").and_then(|from| {
            let col = child(from, "col")
                .and_then(|n| n.text())
                .and_then(|s| s.parse::<u32>().ok())?
                .checked_add(1)?;
            let row = child(from, "row")
                .and_then(|n| n.text())
                .and_then(|s| s.parse::<u32>().ok())?
                .checked_add(1)?;
            if col > 16_384 || row > 1_048_576 {
                return None;
            }
            Some(format!("{}{}", column(col as usize), row))
        });
        for (index, shape) in anchor
            .descendants()
            .filter(|n| element(*n, "sp"))
            .enumerate()
        {
            let Some(body) = child(shape, "txBody") else {
                continue;
            };
            let text = all_text(body, "t");
            if text.is_empty() {
                continue;
            }
            let name = shape
                .descendants()
                .find(|n| element(*n, "cNvPr"))
                .and_then(|n| attr(n, "name"))
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("図形 {}", index + 1));
            let mut location = json!({"sheetName": sheet, "shapeName": name});
            if let Some(anchor) = &position {
                location["anchor"] = json!(anchor);
            }
            let mut unit = Unit::new("shape", location, text).in_part(part);
            unit.meta.anchor = position.clone();
            units.push(unit);
        }
    }
}

fn excel_vml(doc: &Document<'_>, sheet: &str, part: &str, units: &mut Vec<Unit>) {
    for (index, shape) in doc
        .descendants()
        .filter(|n| element(*n, "shape"))
        .enumerate()
    {
        let text: String = shape
            .descendants()
            .find(|n| element(*n, "textbox"))
            .map(|n| {
                n.descendants()
                    .filter(|t| t.is_text())
                    .filter_map(|t| t.text())
                    .collect()
            })
            .unwrap_or_default();
        if text.is_empty() {
            continue;
        }
        let name = attr(shape, "id")
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("図形 {}", index + 1));
        let row = shape
            .descendants()
            .find(|n| element(*n, "Row"))
            .and_then(|n| n.text())
            .and_then(|s| s.parse::<usize>().ok());
        let col = shape
            .descendants()
            .find(|n| element(*n, "Column"))
            .and_then(|n| n.text())
            .and_then(|s| s.parse::<usize>().ok());
        let mut location = json!({"sheetName": sheet, "shapeName": name});
        if let (Some(row), Some(col)) = (
            row.and_then(|value| value.checked_add(1)),
            col.and_then(|value| value.checked_add(1)),
        ) {
            if row <= 1_048_576 && col <= 16_384 {
                location["anchor"] = json!(format!("{}{}", column(col), row));
            }
        }
        let mut unit = Unit::new("shape", location, text).in_part(part);
        unit.meta.anchor = unit.location["anchor"].as_str().map(str::to_owned);
        units.push(unit);
    }
}

fn column(mut number: usize) -> String {
    let mut name = String::new();
    while number > 0 {
        number -= 1;
        name.insert(0, (b'A' + (number % 26) as u8) as char);
        number /= 26;
    }
    name
}

fn powerpoint_units(archive: &HashMap<String, String>) -> Result<Vec<Unit>, ExtractError> {
    let presentation_part = main_part(archive, "presentation.xml")?;
    let presentation = parsed(archive, &presentation_part)?;
    let rels = relationships(archive, &presentation_part)?;
    let mut units = Vec::new();
    for (i, slide) in presentation
        .descendants()
        .filter(|n| element(*n, "sldId"))
        .enumerate()
    {
        let id = attr(slide, "id")
            .ok_or_else(|| ExtractError::unreadable("スライドの参照 ID がありません。"))?;
        let part = rels
            .get(id)
            .filter(|(_, kind)| kind.ends_with("/slide"))
            .map(|(part, _)| part)
            .ok_or_else(|| ExtractError::unreadable("スライドの参照先がありません。"))?;
        let doc = parsed(archive, part)?;
        for (index, shape) in doc.descendants().filter(|n| element(*n, "sp")).enumerate() {
            let Some(body) = child(shape, "txBody") else {
                continue;
            };
            let text = all_text(body, "t");
            if text.is_empty() {
                continue;
            }
            let name = shape
                .descendants()
                .find(|n| element(*n, "cNvPr"))
                .and_then(|n| attr(n, "name"))
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("図形 {}", index + 1));
            units.push(
                Unit::new(
                    "shape",
                    json!({"slideNumber": i + 1, "shapeName": name}),
                    text,
                )
                .in_part(part),
            );
        }
        for (index, frame) in doc
            .descendants()
            .filter(|n| element(*n, "graphicFrame"))
            .enumerate()
        {
            let Some(table) = frame.descendants().find(|n| element(*n, "tbl")) else {
                continue;
            };
            let name = frame
                .descendants()
                .find(|n| element(*n, "cNvPr"))
                .and_then(|n| attr(n, "name"))
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("表 {}", index + 1));
            for (row, tr) in table.children().filter(|n| element(*n, "tr")).enumerate() {
                for (col, cell) in tr.children().filter(|n| element(*n, "tc")).enumerate() {
                    let text = all_text(cell, "t");
                    if !text.is_empty() {
                        units.push(Unit::new("slideTableCell", json!({"slideNumber": i + 1, "tableName": name, "row": row + 1, "column": col + 1}), text).in_part(part));
                    }
                }
            }
        }
    }
    Ok(units)
}

fn word_units(archive: &HashMap<String, String>) -> Result<Vec<Unit>, ExtractError> {
    let part = main_part(archive, "document.xml")?;
    let doc = parsed(archive, &part)?;
    let body = doc
        .descendants()
        .find(|n| element(*n, "body"))
        .ok_or_else(|| ExtractError::unreadable("Word 本文がありません。"))?;
    let mut units = Vec::new();
    word_block(body, &[], &part, &mut units)?;
    Ok(units)
}

fn word_block(
    container: Node<'_, '_>,
    path: &[Value],
    part: &str,
    units: &mut Vec<Unit>,
) -> Result<(), ExtractError> {
    let mut paragraph = 0;
    let mut table = 0;
    for node in container.children().filter(|n| n.is_element()) {
        if element(node, "p") {
            paragraph += 1;
            let text = all_text(node, "t");
            if text.is_empty() {
                continue;
            }
            if path.is_empty() {
                units.push(
                    Unit::new("paragraph", json!({"paragraphNumber": paragraph}), text)
                        .in_part(part),
                );
            } else {
                units.push(
                    Unit::new(
                        "wordTableParagraph",
                        json!({"tablePath": path, "paragraphNumber": paragraph}),
                        text,
                    )
                    .in_part(part),
                );
            }
        } else if element(node, "tbl") {
            if path.len() >= 16 {
                return Err(ExtractError::limit("Word の入れ子表が深すぎます。"));
            }
            table += 1;
            for (row, tr) in node.children().filter(|n| element(*n, "tr")).enumerate() {
                for (col, cell) in tr.children().filter(|n| element(*n, "tc")).enumerate() {
                    let mut nested_path = path.to_vec();
                    nested_path.push(json!({"table": table, "row": row + 1, "column": col + 1}));
                    word_block(cell, &nested_path, part, units)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excel_coordinates_and_rows_are_sparse_and_distinct() {
        assert_eq!(parse_cell_address("A1"), Some((1, 1)));
        assert_eq!(parse_cell_address("XFD1048576"), Some((1_048_576, 16_384)));
        assert_eq!(parse_cell_address("XFE1"), None);
        assert_eq!(parse_cell_address("A0"), None);
        let mut units = vec![
            Unit::excel_cell("xl/worksheets/sheet1.xml", "Sheet1", "A12", "顧客".into()),
            Unit::excel_cell("xl/worksheets/sheet1.xml", "Sheet1", "D12", "必須".into()),
            Unit::excel_cell("xl/worksheets/sheet2.xml", "Sheet2", "A12", "別".into()),
        ];
        assign_keys(&mut units);
        assert_eq!(units[0].meta.group_key, units[1].meta.group_key);
        assert_ne!(units[0].meta.group_key, units[2].meta.group_key);
        assert_ne!(units[0].meta.unit_key, units[1].meta.unit_key);
        assert_eq!(units[1].meta.column, Some(4));
    }

    #[test]
    fn sheet_metadata_keeps_merge_and_hidden_ranges_without_expansion() {
        let xml = r#"<worksheet><cols><col min="3" max="100" hidden="1"/></cols><sheetData><row r="12" hidden="1"/></sheetData><mergeCells><mergeCell ref="A12:D12"/></mergeCells></worksheet>"#;
        let document = Document::parse(xml).unwrap();
        let meta = sheet_meta(&document, "xl/worksheets/sheet1.xml", "Sheet1");
        assert_eq!(meta.merge_ranges.len(), 1);
        assert_eq!(meta.merge_ranges[0].last_column, 4);
        assert_eq!(meta.hidden_rows, vec![12]);
        assert_eq!(meta.hidden_columns.len(), 1);
        assert_eq!(meta.hidden_columns[0].last, 100);
    }

    #[test]
    fn text_units_decode_utf8_bom_and_shift_jis_without_replacement() {
        let utf8 = text_units("\u{feff}日本語\n".as_bytes()).unwrap();
        assert_eq!(utf8[0].text, "日本語");
        let shift_jis =
            text_units(&[0x93, 0xfa, 0x96, 0x7b, 0x8c, 0xea, 0x87, 0x40, b'\r', b'\n']).unwrap();
        assert_eq!(shift_jis[0].text, "日本語①");
        assert_eq!(shift_jis[0].location, json!({"lineNumber": 1}));
        assert_eq!(
            text_units(b"\xef\xbb\xbf\xff").unwrap_err().code,
            "unsupportedEncoding"
        );
        assert_eq!(
            text_units(b"\xff\xfe").unwrap_err().code,
            "unsupportedEncoding"
        );
    }

    #[test]
    fn deeply_nested_word_tables_hit_resource_limit() {
        let mut xml = String::from("<body>");
        for _ in 0..17 {
            xml.push_str("<tbl><tr><tc>");
        }
        xml.push_str("<p><t>text</t></p>");
        for _ in 0..17 {
            xml.push_str("</tc></tr></tbl>");
        }
        xml.push_str("</body>");
        let document = Document::parse(&xml).expect("valid test XML");
        let error = word_block(
            document.root_element(),
            &[],
            "word/document.xml",
            &mut Vec::new(),
        )
        .unwrap_err();
        assert_eq!(error.code, "resourceLimit");
    }
}
