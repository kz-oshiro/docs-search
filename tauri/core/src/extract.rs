use encoding_rs::SHIFT_JIS;
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;
use zip::ZipArchive;

const MAX_PART: u64 = 32 * 1024 * 1024;
const MAX_TOTAL: u64 = 128 * 1024 * 1024;
const MAX_PARTS: usize = 4096;

type Archive = HashMap<String, Result<String, ExtractError>>;

#[derive(Clone, Copy, Debug, Default)]
pub struct ExtractionOptions {
    pub include_notes: bool,
    pub include_formulas: bool,
}

impl ExtractionOptions {
    pub fn scope(self) -> i64 {
        i64::from(self.include_notes) | (i64::from(self.include_formulas) << 1)
    }
}

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
    #[serde(default)]
    pub layout: SheetLayout,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetLayout {
    pub default_row_height: Option<f64>,
    pub default_column_width: Option<f64>,
    pub rows: Vec<RowLayout>,
    pub columns: Vec<ColumnLayout>,
    pub cells: Vec<CellLayout>,
    pub styles: Vec<CellStyle>,
    pub notes: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowLayout {
    pub row: u32,
    pub height: Option<f64>,
    pub style: Option<usize>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnLayout {
    pub first: u32,
    pub last: u32,
    pub width: Option<f64>,
    pub style: Option<usize>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CellLayout {
    pub row: u32,
    pub column: u32,
    pub style: usize,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CellStyle {
    pub horizontal: Option<String>,
    pub vertical: Option<String>,
    pub wrap: bool,
    pub font_name: Option<String>,
    pub font_size: Option<f64>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub color: Option<String>,
    pub fill: Option<String>,
    pub borders: Vec<Option<String>>,
}

fn positive_number(node: Node<'_, '_>, name: &str) -> Option<f64> {
    attr(node, name)?
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
}
fn rgb_color(node: Node<'_, '_>, notes: &mut HashSet<String>) -> Option<String> {
    if attr(node, "theme").is_some()
        || attr(node, "indexed").is_some()
        || attr(node, "tint").is_some()
    {
        notes.insert("テーマ色・パレット色・濃淡調整は完全再現できません。".into());
    }
    if let Some(rgb) = attr(node, "rgb") {
        if rgb.len() == 8 && rgb.is_ascii() && rgb.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Some(format!("#{}", &rgb[2..]));
        }
        if rgb.len() == 6 && rgb.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Some(format!("#{rgb}"));
        }
    }
    None
}
fn excel_styles(
    archive: &Archive,
    rels: &HashMap<String, (String, String)>,
) -> (Vec<CellStyle>, Vec<String>) {
    let Some((part, _)) = rels.values().find(|(_, kind)| kind.ends_with("/styles")) else {
        return (
            Vec::new(),
            vec!["保存されたセル書式がないため標準表示です。".into()],
        );
    };
    let Ok(doc) = parsed(archive, part) else {
        return (
            Vec::new(),
            vec!["セル書式を読み取れないため標準表示です。".into()],
        );
    };
    let root = doc.root_element();
    let nodes = |name| {
        child(root, name)
            .map(|node| {
                node.children()
                    .filter(|n| n.is_element())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let fonts = nodes("fonts");
    let fills = nodes("fills");
    let borders = nodes("borders");
    let bases = nodes("cellStyleXfs");
    let mut notes = HashSet::new();
    let mut styles = Vec::new();
    for xf in nodes("cellXfs") {
        let base = attr(xf, "xfId")
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|index| bases.get(index).copied());
        let reference = |id, apply| {
            let value = if attr(xf, apply) == Some("0") {
                base.and_then(|node| attr(node, id))
            } else {
                attr(xf, id).or_else(|| base.and_then(|node| attr(node, id)))
            };
            value
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0)
        };
        let mut style = CellStyle::default();
        let alignment = if attr(xf, "applyAlignment") == Some("0") {
            base.and_then(|node| child(node, "alignment"))
        } else {
            child(xf, "alignment").or_else(|| base.and_then(|node| child(node, "alignment")))
        };
        if let Some(alignment) = alignment {
            style.horizontal = attr(alignment, "horizontal")
                .filter(|value| ["left", "center", "right", "justify"].contains(value))
                .map(str::to_owned);
            style.vertical = attr(alignment, "vertical")
                .filter(|value| ["top", "center", "bottom"].contains(value))
                .map(str::to_owned);
            style.wrap = matches!(attr(alignment, "wrapText"), Some("1" | "true"));
            if attr(alignment, "textRotation").is_some_and(|value| value != "0")
                || matches!(attr(alignment, "shrinkToFit"), Some("1" | "true"))
            {
                notes.insert("文字回転・縮小表示は再現しません。".into());
            }
        }
        if let Some(font) = fonts.get(reference("fontId", "applyFont")) {
            let enabled = |name| {
                child(*font, name)
                    .is_some_and(|node| !matches!(attr(node, "val"), Some("0" | "false" | "none")))
            };
            style.bold = enabled("b");
            style.italic = enabled("i");
            style.underline = enabled("u");
            style.strike = enabled("strike");
            style.font_size = child(*font, "sz").and_then(|node| positive_number(node, "val"));
            style.font_name = child(*font, "name")
                .and_then(|node| attr(node, "val"))
                .map(str::to_owned);
            style.color = child(*font, "color").and_then(|node| rgb_color(node, &mut notes));
        }
        if let Some(fill) = fills.get(reference("fillId", "applyFill")) {
            if let Some(pattern) = child(*fill, "patternFill") {
                if attr(pattern, "patternType") == Some("solid") {
                    style.fill =
                        child(pattern, "fgColor").and_then(|node| rgb_color(node, &mut notes));
                } else if !matches!(attr(pattern, "patternType"), None | Some("none")) {
                    notes.insert("パターン・グラデーションの塗りつぶしは再現しません。".into());
                }
            }
            if child(*fill, "gradientFill").is_some() {
                notes.insert("パターン・グラデーションの塗りつぶしは再現しません。".into());
            }
        }
        if let Some(border) = borders.get(reference("borderId", "applyBorder")) {
            for side in ["top", "right", "bottom", "left"] {
                let css = child(*border, side).and_then(|node| {
                    let (width, line) = match attr(node, "style")? {
                        "thin" => (1, "solid"),
                        "hair" | "dotted" => (1, "dotted"),
                        "dashed" | "dashDot" | "dashDotDot" => (1, "dashed"),
                        "medium" => (2, "solid"),
                        "thick" => (3, "solid"),
                        "double" => (3, "double"),
                        "mediumDashed" | "mediumDashDot" | "mediumDashDotDot" | "slantDashDot" => {
                            (2, "dashed")
                        }
                        _ => return None,
                    };
                    let color = child(node, "color")
                        .and_then(|color| rgb_color(color, &mut notes))
                        .unwrap_or_else(|| "#64748b".into());
                    Some(format!("{width}px {line} {color}"))
                });
                style
                    .borders
                    .push(Some(css.unwrap_or_else(|| "none".into())));
            }
            if child(*border, "diagonal").is_some_and(|node| attr(node, "style").is_some()) {
                notes.insert("斜め罫線は再現しません。".into());
            }
        }
        styles.push(style);
    }
    let mut notes: Vec<_> = notes.into_iter().collect();
    notes.sort();
    (styles, notes)
}

fn sheet_layout(doc: &Document<'_>, styles: &[CellStyle], notes: &[String]) -> SheetLayout {
    let format = doc
        .descendants()
        .find(|node| element(*node, "sheetFormatPr"));
    let mut layout = SheetLayout {
        default_row_height: format.and_then(|node| positive_number(node, "defaultRowHeight")),
        default_column_width: format.and_then(|node| positive_number(node, "defaultColWidth")),
        styles: styles.to_vec(),
        notes: notes.to_vec(),
        ..Default::default()
    };
    for row in doc.descendants().filter(|node| element(*node, "row")) {
        if let Some(number) = attr(row, "r")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|number| (1..=1_048_576).contains(number))
        {
            layout.rows.push(RowLayout {
                row: number,
                height: positive_number(row, "ht"),
                style: attr(row, "s").and_then(|value| value.parse().ok()),
            });
        }
    }
    for column in doc.descendants().filter(|node| element(*node, "col")) {
        if let (Some(first), Some(last)) = (
            attr(column, "min").and_then(|value| value.parse::<u32>().ok()),
            attr(column, "max").and_then(|value| value.parse::<u32>().ok()),
        ) {
            if first > 0 && first <= last && last <= 16_384 {
                layout.columns.push(ColumnLayout {
                    first,
                    last,
                    width: positive_number(column, "width"),
                    style: attr(column, "style").and_then(|value| value.parse().ok()),
                });
            }
        }
    }
    let row_styles: HashMap<_, _> = layout
        .rows
        .iter()
        .filter_map(|item| Some((item.row, item.style?)))
        .collect();
    let mut column_styles = vec![None; 16_385];
    for column in &layout.columns {
        for number in column.first..=column.last {
            column_styles[number as usize] = column.style;
        }
    }
    for cell in doc.descendants().filter(|node| element(*node, "c")) {
        if let Some((row, column)) = attr(cell, "r").and_then(parse_cell_address) {
            let style = attr(cell, "s")
                .and_then(|value| value.parse().ok())
                .or_else(|| row_styles.get(&row).copied())
                .or(column_styles[column as usize])
                .unwrap_or(0);
            layout.cells.push(CellLayout { row, column, style });
        }
    }
    if doc
        .descendants()
        .any(|node| element(node, "conditionalFormatting"))
    {
        layout
            .notes
            .push("条件付き書式は計算しません。保存された通常書式を表示します。".into());
    }
    layout.notes.push("幅と高さは近似です。数値書式・リッチテキストの部分書式・画面外の結合範囲は完全再現しません。".into());
    layout
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
    pub issues: Vec<ExtractError>,
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

#[derive(Clone, Debug)]
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
// Phonetic runs are auxiliary metadata, not the displayed cell string.
fn excel_visible_text(node: Node<'_, '_>) -> String {
    node.descendants()
        .filter(|n| element(*n, "t"))
        .filter(|n| {
            !n.ancestors()
                .any(|a| element(a, "rPh") || element(a, "phoneticPr"))
        })
        .filter_map(|n| n.text())
        .collect()
}
fn parsed<'a>(archive: &'a Archive, part: &str) -> Result<Document<'a>, ExtractError> {
    let xml = archive
        .get(part)
        .ok_or_else(|| ExtractError::unreadable(format!("必要な部品がありません: {part}")))?
        .as_ref()
        .map_err(Clone::clone)?;
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
    archive: &Archive,
    source: &str,
) -> Result<HashMap<String, (String, String)>, ExtractError> {
    relationships_of(archive, source, &[])
}

fn relationships_of(
    archive: &Archive,
    source: &str,
    kinds: &[&str],
) -> Result<HashMap<String, (String, String)>, ExtractError> {
    let part = rels_part(source);
    if !archive.contains_key(&part) {
        return Ok(HashMap::new());
    }
    let doc = parsed(archive, &part)?;
    let mut map = HashMap::new();
    for rel in doc.descendants().filter(|n| element(*n, "Relationship")) {
        if !kinds.is_empty()
            && !attr(rel, "Type").is_some_and(|kind| {
                kinds
                    .iter()
                    .any(|suffix| kind.ends_with(&format!("/{suffix}")))
            })
        {
            continue;
        }
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

fn main_part(archive: &Archive, suffix: &str) -> Result<String, ExtractError> {
    let doc = parsed(archive, "_rels/.rels")?;
    for rel in doc.descendants().filter(|n| element(*n, "Relationship")) {
        if attr(rel, "Type").is_some_and(|s| s.ends_with("/officeDocument"))
            && attr(rel, "TargetMode") != Some("External")
        {
            let path = normalize_part("", attr(rel, "Target").unwrap_or(""))?;
            if element(
                parsed(archive, &path)?.root_element(),
                suffix.trim_end_matches(".xml"),
            ) {
                return Ok(path);
            }
        }
    }
    Err(ExtractError::unreadable("主文書部品がありません。"))
}

fn read_archive<R: Read + Seek>(reader: R) -> Result<Archive, ExtractError> {
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
        if total.saturating_add(item.size()) > MAX_TOTAL {
            return Err(ExtractError::limit(
                "Office ファイルの展開量が上限を超えました。",
            ));
        }
        // Decode faults stay with their part so optional annotation faults preserve the body.
        // Package-wide limits and invalid ZIP structure still reject the complete file.
        total += item.size();
        let content = if item.size() > MAX_PART {
            Err(ExtractError::limit("Office 部品が上限を超えました。"))
        } else {
            let mut bytes = Vec::with_capacity(item.size() as usize);
            let read = item
                .by_ref()
                .take(MAX_PART + 1)
                .read_to_end(&mut bytes)
                .map_err(ExtractError::from);
            total = total.saturating_add((bytes.len() as u64).saturating_sub(item.size()));
            read.and_then(|_| {
                if bytes.len() as u64 > MAX_PART {
                    Err(ExtractError::limit("Office 部品が上限を超えました。"))
                } else {
                    String::from_utf8(bytes).map_err(|_| {
                        ExtractError::unreadable("Office XML が UTF-8 ではありません。")
                    })
                }
            })
        };
        if total > MAX_TOTAL {
            return Err(ExtractError::limit(
                "Office ファイルの展開量が上限を超えました。",
            ));
        }
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
    office_units_with_options(path, file_type, ExtractionOptions::default())
}

pub fn office_units_with_options(
    path: &Path,
    file_type: &str,
    options: ExtractionOptions,
) -> Result<ExtractedDocument, ExtractError> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_TOTAL {
        return Err(ExtractError::limit(
            "Office ファイルがサイズ上限を超えました。",
        ));
    }
    let archive = read_archive(File::open(path)?)?;
    let mut document = match file_type {
        "xlsx" | "xlsm" => excel_units(&archive, options),
        "pptx" => powerpoint_units(&archive, options),
        "docx" => word_units(&archive, options),
        _ => Err(ExtractError::unreadable("対応していない形式です。")),
    }?;
    assign_keys(&mut document.units);
    Ok(document)
}

fn excel_units(
    archive: &Archive,
    options: ExtractionOptions,
) -> Result<ExtractedDocument, ExtractError> {
    let workbook_part = main_part(archive, "workbook.xml")?;
    let workbook = parsed(archive, &workbook_part)?;
    let rels = relationships(archive, &workbook_part)?;
    let (styles, style_notes) = excel_styles(archive, &rels);
    let mut strings = Vec::new();
    if let Some((part, _)) = rels
        .values()
        .find(|(_, kind)| kind.ends_with("/sharedStrings"))
    {
        let doc = parsed(archive, part)?;
        for si in doc.descendants().filter(|n| element(*n, "si")) {
            strings.push(excel_visible_text(si));
        }
    }
    let mut units = Vec::new();
    let mut sheets = Vec::new();
    let mut issues = Vec::new();
    let mut seen_notes = HashSet::new();
    for (sheet_index, sheet) in workbook
        .descendants()
        .filter(|n| element(*n, "sheet"))
        .enumerate()
    {
        let first_unit = units.len();
        let name = attr(sheet, "name").unwrap_or("(名称なし)");
        let id = attr(sheet, "id")
            .ok_or_else(|| ExtractError::unreadable("シートの参照 ID がありません。"))?;
        let part = rels
            .get(id)
            .filter(|(_, kind)| kind.ends_with("/worksheet"))
            .map(|(part, _)| part)
            .ok_or_else(|| ExtractError::unreadable("シートの参照先がありません。"))?;
        let doc = parsed(archive, part)?;
        let mut metadata = sheet_meta(&doc, part, name);
        metadata.layout = sheet_layout(&doc, &styles, &style_notes);
        sheets.push(metadata);
        for cell in doc.descendants().filter(|n| element(*n, "c")) {
            let address = attr(cell, "r").unwrap_or("");
            if address.is_empty() {
                continue;
            }
            if options.include_formulas {
                if let Some(formula) = child(cell, "f") {
                    let text = formula.text().unwrap_or("");
                    if !text.is_empty() {
                        let mut unit = Unit::excel_cell(part, name, address, format!("={text}"));
                        unit.source_kind = "formula";
                        unit.meta.content_class = "formula".into();
                        unit.location["formulaType"] =
                            json!(attr(formula, "t").unwrap_or("normal"));
                        if let Some(shared) = attr(formula, "si") {
                            unit.location["sharedIndex"] = json!(shared);
                        }
                        if let Some(range) = attr(formula, "ref") {
                            unit.location["formulaRange"] = json!(range);
                        }
                        units.push(unit);
                    }
                }
            }
            let kind = attr(cell, "t").unwrap_or("n");
            let value = if kind == "inlineStr" {
                child(cell, "is")
                    .map(excel_visible_text)
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
            let mut unit = Unit::excel_cell(part, name, address, value);
            if let Some(formula) = child(cell, "f") {
                if let Some(shared) = attr(formula, "si") {
                    unit.location["sharedIndex"] = json!(shared);
                }
            }
            units.push(unit);
        }
        let mut comment_addresses = HashSet::new();
        if options.include_notes {
            for note_part in annotation_parts(
                archive,
                part,
                &["comments", "threadedComment"],
                None,
                &mut issues,
            ) {
                if seen_notes.insert(note_part.clone()) {
                    match excel_comments(archive, &note_part, name) {
                        Ok(notes) => {
                            comment_addresses.extend(notes.iter().filter_map(|unit| {
                                unit.location["cellAddress"].as_str().map(str::to_owned)
                            }));
                            units.extend(notes);
                        }
                        Err(error) => part_issue(&mut issues, &note_part, error),
                    }
                }
            }
        }
        let has_drawings = doc
            .descendants()
            .any(|n| element(n, "drawing") || element(n, "legacyDrawing"));
        let sheet_rels = if has_drawings {
            relationships_of(archive, part, &["drawing", "vmlDrawing"])?
        } else {
            HashMap::new()
        };
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
                excel_vml(&drawing, name, drawing_part, &comment_addresses, &mut units);
            } else {
                excel_drawing(&drawing, name, drawing_part, &mut units);
            }
        }
        for unit in &mut units[first_unit..] {
            unit.location["sheetIndex"] = json!(sheet_index);
        }
    }
    Ok(ExtractedDocument {
        units,
        sheets,
        issues,
    })
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

fn excel_vml(
    doc: &Document<'_>,
    sheet: &str,
    part: &str,
    comment_addresses: &HashSet<String>,
    units: &mut Vec<Unit>,
) {
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
        if shape
            .descendants()
            .any(|n| element(n, "ClientData") && attr(n, "ObjectType") == Some("Note"))
            && location["anchor"]
                .as_str()
                .is_some_and(|address| comment_addresses.contains(address))
        {
            continue;
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

fn powerpoint_units(
    archive: &Archive,
    options: ExtractionOptions,
) -> Result<ExtractedDocument, ExtractError> {
    let presentation_part = main_part(archive, "presentation.xml")?;
    let presentation = parsed(archive, &presentation_part)?;
    let rels = relationships(archive, &presentation_part)?;
    let mut units = Vec::new();
    let mut issues = Vec::new();
    let mut seen_notes = HashSet::new();
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
        if options.include_notes {
            for note_part in annotation_parts(archive, part, &["notesSlide"], None, &mut issues) {
                if seen_notes.insert(note_part.clone()) {
                    match powerpoint_notes(archive, &note_part, i + 1) {
                        Ok(notes) => units.extend(notes),
                        Err(error) => part_issue(&mut issues, &note_part, error),
                    }
                }
            }
        }
    }
    Ok(ExtractedDocument {
        units,
        issues,
        ..ExtractedDocument::default()
    })
}

fn word_units(
    archive: &Archive,
    options: ExtractionOptions,
) -> Result<ExtractedDocument, ExtractError> {
    let part = main_part(archive, "document.xml")?;
    let doc = parsed(archive, &part)?;
    let body = doc
        .descendants()
        .find(|n| element(*n, "body"))
        .ok_or_else(|| ExtractError::unreadable("Word 本文がありません。"))?;
    let mut units = Vec::new();
    word_block(body, &[], &part, &mut units)?;
    let mut issues = Vec::new();
    if options.include_notes {
        let mut seen = HashSet::new();
        for reference in doc
            .descendants()
            .filter(|n| element(*n, "headerReference") || element(*n, "footerReference"))
        {
            let kind = if element(reference, "headerReference") {
                "header"
            } else {
                "footer"
            };
            let Some(id) = attr(reference, "id") else {
                part_issue(
                    &mut issues,
                    &part,
                    ExtractError::unreadable("ヘッダー・フッターの参照IDがありません。"),
                );
                continue;
            };
            let parts = annotation_parts(archive, &part, &[kind], Some(id), &mut issues);
            for note_part in parts {
                if seen.insert(note_part.clone()) {
                    let result = word_annotation(
                        archive,
                        &note_part,
                        kind,
                        attr(reference, "type").unwrap_or("default"),
                    );
                    match result {
                        Ok(notes) => units.extend(notes),
                        Err(error) => part_issue(&mut issues, &note_part, error),
                    }
                }
            }
        }
        for note_part in annotation_parts(archive, &part, &["comments"], None, &mut issues) {
            if seen.insert(note_part.clone()) {
                match word_annotation(archive, &note_part, "comment", "") {
                    Ok(notes) => units.extend(notes),
                    Err(error) => part_issue(&mut issues, &note_part, error),
                }
            }
        }
    }
    Ok(ExtractedDocument {
        units,
        issues,
        ..ExtractedDocument::default()
    })
}

fn part_issue(issues: &mut Vec<ExtractError>, part: &str, mut error: ExtractError) {
    error.message = format!("注記部品 {part}: {}", error.message);
    if !issues
        .iter()
        .any(|previous| previous.code == error.code && previous.message == error.message)
    {
        issues.push(error);
    }
}

fn annotation_parts(
    archive: &Archive,
    source: &str,
    kinds: &[&str],
    id: Option<&str>,
    issues: &mut Vec<ExtractError>,
) -> Vec<String> {
    let rel_part = rels_part(source);
    if !archive.contains_key(&rel_part) {
        if id.is_some() {
            part_issue(
                issues,
                &rel_part,
                ExtractError::unreadable("注記の参照先がありません。"),
            );
        }
        return Vec::new();
    }
    let doc = match parsed(archive, &rel_part) {
        Ok(doc) => doc,
        Err(error) => {
            part_issue(issues, &rel_part, error);
            return Vec::new();
        }
    };
    let mut parts = Vec::new();
    let mut found = false;
    for rel in doc.descendants().filter(|n| element(*n, "Relationship")) {
        if id.is_some_and(|id| attr(rel, "Id") != Some(id)) {
            continue;
        }
        if !attr(rel, "Type").is_some_and(|kind| {
            kinds
                .iter()
                .any(|suffix| kind.ends_with(&format!("/{suffix}")))
        }) {
            continue;
        }
        found = true;
        let target = attr(rel, "Target").unwrap_or("");
        if attr(rel, "TargetMode") == Some("External") || target.is_empty() {
            part_issue(
                issues,
                &rel_part,
                ExtractError::unreadable(format!("外部または空の注記参照は取得しません: {target}")),
            );
            continue;
        }
        match normalize_part(source, target) {
            Ok(part) => {
                if !parts.contains(&part) {
                    parts.push(part);
                }
            }
            Err(error) => part_issue(issues, &rel_part, error),
        }
    }
    if id.is_some() && !found {
        part_issue(
            issues,
            &rel_part,
            ExtractError::unreadable("注記の参照IDに対応する部品がありません。"),
        );
    }
    parts
}

fn excel_comments(archive: &Archive, part: &str, sheet: &str) -> Result<Vec<Unit>, ExtractError> {
    let doc = parsed(archive, part)?;
    if !matches!(
        doc.root_element().tag_name().name(),
        "comments" | "ThreadedComments"
    ) {
        return Err(ExtractError::unreadable(
            "Excelコメントの部品形式が不正です。",
        ));
    }
    let comments: Vec<_> = doc
        .descendants()
        .filter(|n| element(*n, "comment") || element(*n, "threadedComment"))
        .collect();
    let mut addresses: HashMap<String, String> = comments
        .iter()
        .filter_map(|&node| Some((attr(node, "id")?.into(), attr(node, "ref")?.into())))
        .collect();
    // Replies can inherit a reference through a parent chain; no cell/formula is synthesized.
    for _ in 0..comments.len().min(16) {
        let mut changed = false;
        for &node in &comments {
            if let (Some(id), Some(parent)) = (attr(node, "id"), attr(node, "parentId")) {
                if !addresses.contains_key(id) {
                    if let Some(address) = addresses.get(parent).cloned() {
                        addresses.insert(id.into(), address);
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut units = Vec::new();
    for (index, node) in comments.into_iter().enumerate() {
        let threaded = element(node, "threadedComment");
        let text = if threaded {
            child(node, "text")
                .map(|text| {
                    text.descendants()
                        .filter(|n| n.is_text())
                        .filter_map(|n| n.text())
                        .collect::<String>()
                })
                .unwrap_or_default()
        } else {
            all_text(node, "t")
        };
        if text.is_empty() {
            continue;
        }
        let id = attr(node, "id")
            .map(str::to_owned)
            .unwrap_or_else(|| (index + 1).to_string());
        let address = attr(node, "ref")
            .map(str::to_owned)
            .or_else(|| addresses.get(&id).cloned());
        let mut location = json!({"sheetName":sheet, "commentId":id, "commentType":if threaded {"threaded"} else {"legacy"}, "partName":part});
        if let Some(address) = &address {
            location["cellAddress"] = json!(address);
        }
        let mut unit = Unit::new("excelComment", location, text).in_part(part);
        unit.meta.content_class = "note".into();
        if let Some((row, column)) = address.as_deref().and_then(parse_cell_address) {
            unit.meta.row = Some(row);
            unit.meta.column = Some(column);
        }
        units.push(unit);
    }
    Ok(units)
}

fn powerpoint_notes(
    archive: &Archive,
    part: &str,
    slide: usize,
) -> Result<Vec<Unit>, ExtractError> {
    let doc = parsed(archive, part)?;
    if !element(doc.root_element(), "notes") {
        return Err(ExtractError::unreadable(
            "PowerPointノートの部品形式が不正です。",
        ));
    }
    let mut units = Vec::new();
    for (index, shape) in doc.descendants().filter(|n| element(*n, "sp")).enumerate() {
        let placeholder = shape
            .descendants()
            .find(|n| element(*n, "ph"))
            .and_then(|n| attr(n, "type"));
        if matches!(
            placeholder,
            Some("dt" | "ftr" | "hdr" | "sldNum" | "sldImg")
        ) {
            continue;
        }
        let Some(body) = child(shape, "txBody") else {
            continue;
        };
        let identity = shape.descendants().find(|n| element(*n, "cNvPr"));
        let name = identity.and_then(|n| attr(n, "name")).unwrap_or("ノート");
        let id = identity
            .and_then(|n| attr(n, "id"))
            .map(str::to_owned)
            .unwrap_or_else(|| (index + 1).to_string());
        for (paragraph, node) in body.children().filter(|n| element(*n, "p")).enumerate() {
            let text = all_text(node, "t");
            if text.is_empty() {
                continue;
            }
            let mut unit = Unit::new("note", json!({"slideNumber":slide, "shapeName":name, "shapeId":id, "paragraphNumber":paragraph+1, "partName":part}), text).in_part(part);
            unit.meta.content_class = "note".into();
            units.push(unit);
        }
    }
    Ok(units)
}

fn word_annotation(
    archive: &Archive,
    part: &str,
    kind: &str,
    section: &str,
) -> Result<Vec<Unit>, ExtractError> {
    let doc = parsed(archive, part)?;
    let expected = match kind {
        "header" => "hdr",
        "footer" => "ftr",
        _ => "comments",
    };
    if !element(doc.root_element(), expected) {
        return Err(ExtractError::unreadable("Word注記の部品形式が不正です。"));
    }
    let containers: Vec<_> = if kind == "comment" {
        doc.descendants()
            .filter(|n| element(*n, "comment"))
            .collect()
    } else {
        vec![doc.root_element()]
    };
    let mut units = Vec::new();
    for (index, container) in containers.into_iter().enumerate() {
        let mut block = Vec::new();
        word_block(container, &[], part, &mut block)?;
        for mut unit in block {
            unit.source_kind = match kind {
                "header" => "wordHeader",
                "footer" => "wordFooter",
                _ => "wordComment",
            };
            unit.meta.content_class = "note".into();
            unit.location["partName"] = json!(part);
            if kind == "comment" {
                unit.location["commentId"] = json!(attr(container, "id")
                    .map(str::to_owned)
                    .unwrap_or_else(|| (index + 1).to_string()));
            } else {
                unit.location["sectionType"] = json!(section);
            }
            units.push(unit);
        }
    }
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
    fn cell_rich_text_preserves_displayed_runs_but_excludes_phonetic_metadata() {
        let xml = r#"<si><r><t>顧客</t></r><rPh><t>コキャク</t></rPh><r><t>番号</t></r><phoneticPr/></si>"#;
        let document = Document::parse(xml).unwrap();
        assert_eq!(excel_visible_text(document.root_element()), "顧客番号");
    }

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
