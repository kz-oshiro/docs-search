use crate::extract::{self, CellRange, CellStyle, ColumnLayout, RowLayout, SheetMeta};
use crate::index::{self, Index, DEFAULT_SCOPE};
use crate::SearchHit;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

const LAST_ROW: u32 = 1_048_576;
const LAST_COLUMN: u32 = 16_384;
const WINDOW_SIDE: u32 = 5;
const MAX_MERGES: usize = 25;
const MAX_CELL_CHARACTERS: usize = 300;

#[derive(Clone, Debug)]
pub struct ContextTarget {
    path: PathBuf,
    part_key: String,
    sheet_name: String,
    focus_row: u32,
    focus_column: u32,
    focus_kind: &'static str,
    evidence_addresses: Vec<String>,
    matching_ranges: BTreeMap<(u32, u32), Vec<[usize; 2]>>,
    expected_stamp: Option<(i64, i64)>,
    use_index: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextRange {
    pub first_row: u32,
    pub first_column: u32,
    pub row_count: u32,
    pub column_count: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextCell {
    pub row: u32,
    pub column: u32,
    pub address: String,
    pub text: String,
    pub truncated: bool,
    pub match_ranges: Vec<[usize; 2]>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextMerge {
    pub first_row: u32,
    pub first_column: u32,
    pub last_row: u32,
    pub last_column: u32,
    pub anchor_address: String,
    pub anchor_text: Option<String>,
    pub anchor_truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultContext {
    pub sheet_name: String,
    pub focus_address: String,
    pub focus_kind: &'static str,
    pub evidence_addresses: Vec<String>,
    pub first_row: u32,
    pub first_column: u32,
    pub row_count: u32,
    pub column_count: u32,
    pub cells: Vec<ContextCell>,
    pub merges: Vec<ContextMerge>,
    pub hidden_rows: Vec<u32>,
    pub hidden_columns: Vec<u32>,
    pub layout: ContextLayout,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextLayout {
    pub default_row_height: Option<f64>,
    pub default_column_width: Option<f64>,
    pub rows: Vec<RowLayout>,
    pub columns: Vec<ColumnLayout>,
    pub cells: Vec<ContextCellStyle>,
    pub notes: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextCellStyle {
    pub row: u32,
    pub column: u32,
    pub style: CellStyle,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextError {
    pub code: &'static str,
    pub message: String,
}

impl ContextError {
    fn new(code: &'static str, message: &str) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn changed() -> Self {
        Self::new(
            "changedSinceSearch",
            "検索後にファイルが変更・移動・削除されました。再検索してください。",
        )
    }
}

impl ContextTarget {
    pub fn from_hit(hit: &SearchHit, use_index: bool) -> Option<Self> {
        if !matches!(hit.file_type.as_str(), "xlsx" | "xlsm") {
            return None;
        }
        let sheet_name = hit.location.get("sheetName")?.as_str()?.to_owned();
        let (focus_row, focus_column, focus_kind) = match hit.source_kind.as_str() {
            "cell" => (hit.row?, hit.column?, "cell"),
            "formula" => (hit.row?, hit.column?, "formula"),
            "excelRow" => (hit.row?, hit.column?, "excelRow"),
            "shape" => {
                let (row, column) = extract::parse_cell_address(hit.anchor.as_deref()?)?;
                (row, column, "shape")
            }
            _ => return None,
        };
        if !(1..=LAST_ROW).contains(&focus_row)
            || !(1..=LAST_COLUMN).contains(&focus_column)
            || sheet_name.is_empty()
        {
            return None;
        }
        let path = PathBuf::from(&hit.file_path);
        let evidence_addresses = if focus_kind == "excelRow" {
            hit.evidence
                .iter()
                .filter(|evidence| {
                    matches!(evidence.source_kind.as_str(), "cell" | "formula")
                        && evidence.part_key == hit.part_key
                        && evidence.row == hit.row
                })
                .filter_map(|evidence| Some(cell_address(evidence.row?, evidence.column?)))
                .collect()
        } else {
            Vec::new()
        };
        Some(Self {
            matching_ranges: if hit.evidence.is_empty() {
                if hit.source_kind == "cell" {
                    BTreeMap::from([((focus_row, focus_column), hit.source_match_ranges.clone())])
                } else {
                    BTreeMap::new()
                }
            } else {
                let mut ranges = BTreeMap::<_, Vec<_>>::new();
                for evidence in &hit.evidence {
                    if evidence.source_kind == "cell" && evidence.part_key == hit.part_key {
                        if let (Some(row), Some(column)) = (evidence.row, evidence.column) {
                            ranges
                                .entry((row, column))
                                .or_default()
                                .extend(evidence.source_match_ranges.iter().copied());
                        }
                    }
                }
                for values in ranges.values_mut() {
                    values.sort_unstable();
                    values.dedup();
                }
                ranges
            },
            expected_stamp: index::stamp(&path),
            path,
            part_key: hit.part_key.clone(),
            sheet_name,
            focus_row,
            focus_column,
            focus_kind,
            evidence_addresses,
            use_index,
        })
    }

    fn initial_range(&self) -> ContextRange {
        ContextRange {
            first_row: self.focus_row.saturating_sub(2).clamp(1, LAST_ROW - 4),
            first_column: self
                .focus_column
                .saturating_sub(2)
                .clamp(1, LAST_COLUMN - 4),
            row_count: WINDOW_SIDE,
            column_count: WINDOW_SIDE,
        }
    }

    fn sheet<'a>(&self, sheets: &'a [SheetMeta]) -> Option<&'a SheetMeta> {
        sheets.iter().find(|sheet| {
            sheet.sheet_name == self.sheet_name
                && (self.focus_kind == "shape" || sheet.part_key == self.part_key)
        })
    }
}

fn validate_range(target: &ContextTarget, range: ContextRange) -> Result<(u32, u32), ContextError> {
    if range.row_count == 0
        || range.column_count == 0
        || range.row_count > WINDOW_SIDE
        || range.column_count > WINDOW_SIDE
        || range.first_row == 0
        || range.first_column == 0
    {
        return Err(ContextError::new("invalidRange", "表示範囲が不正です。"));
    }
    let last_row = range
        .first_row
        .checked_add(range.row_count - 1)
        .filter(|last| *last <= LAST_ROW)
        .ok_or_else(|| ContextError::new("invalidRange", "表示範囲が不正です。"))?;
    let last_column = range
        .first_column
        .checked_add(range.column_count - 1)
        .filter(|last| *last <= LAST_COLUMN)
        .ok_or_else(|| ContextError::new("invalidRange", "表示範囲が不正です。"))?;
    if target.focus_row < range.first_row || target.focus_row > last_row {
        return Err(ContextError::new(
            "invalidRange",
            "一致した行を含む範囲を指定してください。",
        ));
    }
    Ok((last_row, last_column))
}

fn intersects(merge: &CellRange, range: ContextRange, last_row: u32, last_column: u32) -> bool {
    merge.first_row <= last_row
        && merge.last_row >= range.first_row
        && merge.first_column <= last_column
        && merge.last_column >= range.first_column
}

fn visible_merges(
    sheet: &SheetMeta,
    range: ContextRange,
    last_row: u32,
    last_column: u32,
) -> Result<Vec<CellRange>, ContextError> {
    let merges: Vec<CellRange> = sheet
        .merge_ranges
        .iter()
        .filter(|merge| intersects(merge, range, last_row, last_column))
        .take(MAX_MERGES + 1)
        .cloned()
        .collect();
    if merges.len() > MAX_MERGES {
        return Err(ContextError::new(
            "resourceLimit",
            "この範囲には結合セルが多すぎます。",
        ));
    }
    Ok(merges)
}

fn is_visible(row: u32, column: u32, range: ContextRange, end: (u32, u32)) -> bool {
    (range.first_row..=end.0).contains(&row) && (range.first_column..=end.1).contains(&column)
}

fn index_data(
    target: &ContextTarget,
    range: ContextRange,
    end: (u32, u32),
) -> Option<(SheetMeta, Vec<CellRange>, Vec<(u32, u32, String)>)> {
    let index = Index::open().ok()?;
    if !index.current(&target.path, DEFAULT_SCOPE) {
        return None;
    }
    let sheets = index.sheet_metadata(&target.path).ok()?;
    let sheet = target.sheet(&sheets)?.clone();
    let merges = visible_merges(&sheet, range, end.0, end.1).ok()?;
    let mut cells = index
        .cells_in_range(
            &target.path,
            &sheet.part_key,
            (range.first_row, end.0),
            (range.first_column, end.1),
        )
        .ok()?;
    let mut seen = HashSet::new();
    for merge in &merges {
        let anchor = (merge.first_row, merge.first_column);
        if !is_visible(anchor.0, anchor.1, range, end) && seen.insert(anchor) {
            cells.extend(
                index
                    .cells_in_range(
                        &target.path,
                        &sheet.part_key,
                        (anchor.0, anchor.0),
                        (anchor.1, anchor.1),
                    )
                    .ok()?,
            );
        }
    }
    Some((sheet, merges, cells))
}

fn direct_data(
    target: &ContextTarget,
    range: ContextRange,
    end: (u32, u32),
) -> Result<(SheetMeta, Vec<CellRange>, Vec<(u32, u32, String)>), ContextError> {
    let file_type = target
        .path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ContextError::new("unreadable", "Excelファイルを読めません。"))?;
    let document =
        extract::office_units(&target.path, &file_type.to_ascii_lowercase()).map_err(|error| {
            ContextError {
                code: error.code,
                message: error.message,
            }
        })?;
    let sheet = target
        .sheet(&document.sheets)
        .ok_or_else(ContextError::changed)?
        .clone();
    let merges = visible_merges(&sheet, range, end.0, end.1)?;
    let anchors: HashSet<(u32, u32)> = merges
        .iter()
        .map(|merge| (merge.first_row, merge.first_column))
        .collect();
    let cells = document
        .units
        .into_iter()
        .filter(|unit| unit.source_kind == "cell" && unit.meta.part_key == sheet.part_key)
        .filter_map(|unit| {
            let position = (unit.meta.row?, unit.meta.column?);
            (is_visible(position.0, position.1, range, end) || anchors.contains(&position))
                .then_some((position.0, position.1, unit.text))
        })
        .collect();
    Ok((sheet, merges, cells))
}

fn clip(value: &str) -> (String, bool) {
    let mut chars = value.chars();
    let clipped: String = chars.by_ref().take(MAX_CELL_CHARACTERS).collect();
    (clipped, chars.next().is_some())
}

fn cell_address(row: u32, mut column: u32) -> String {
    let mut name = String::new();
    while column > 0 {
        column -= 1;
        name.insert(0, (b'A' + (column % 26) as u8) as char);
        column /= 26;
    }
    format!("{name}{row}")
}

fn assemble(
    target: &ContextTarget,
    range: ContextRange,
    end: (u32, u32),
    sheet: SheetMeta,
    merge_ranges: Vec<CellRange>,
    raw_cells: Vec<(u32, u32, String)>,
) -> ResultContext {
    let saved = &sheet.layout;
    let rows: Vec<_> = saved
        .rows
        .iter()
        .filter(|item| (range.first_row..=end.0).contains(&item.row))
        .cloned()
        .collect();
    let columns: Vec<_> = saved
        .columns
        .iter()
        .filter(|item| item.first <= end.1 && item.last >= range.first_column)
        .cloned()
        .collect();
    let style_indices: BTreeMap<_, _> = saved
        .cells
        .iter()
        .filter(|cell| is_visible(cell.row, cell.column, range, end))
        .map(|cell| ((cell.row, cell.column), cell.style))
        .collect();
    let mut formats = Vec::new();
    for row in range.first_row..=end.0 {
        for column in range.first_column..=end.1 {
            let index = style_indices
                .get(&(row, column))
                .copied()
                .or_else(|| {
                    rows.iter()
                        .find(|item| item.row == row)
                        .and_then(|item| item.style)
                })
                .or_else(|| {
                    columns
                        .iter()
                        .rev()
                        .find(|item| item.first <= column && column <= item.last)
                        .and_then(|item| item.style)
                })
                .unwrap_or(0);
            if let Some(style) = saved.styles.get(index) {
                formats.push(ContextCellStyle {
                    row,
                    column,
                    style: style.clone(),
                });
            }
        }
    }
    let layout = ContextLayout {
        default_row_height: saved.default_row_height,
        default_column_width: saved.default_column_width,
        rows,
        columns,
        cells: formats,
        notes: saved.notes.clone(),
    };
    let mut values = BTreeMap::new();
    for (row, column, text) in raw_cells {
        values.entry((row, column)).or_insert(text);
    }
    let mut cells = Vec::new();
    for (&(row, column), text) in &values {
        if is_visible(row, column, range, end) {
            let (text, truncated) = clip(text);
            cells.push(ContextCell {
                match_ranges: target
                    .matching_ranges
                    .get(&(row, column))
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|range| range[1] <= text.chars().count())
                    .collect(),
                row,
                column,
                address: cell_address(row, column),
                text,
                truncated,
            });
        }
    }
    let merges = merge_ranges
        .into_iter()
        .map(|merge| {
            let anchor = (merge.first_row, merge.first_column);
            let (anchor_text, anchor_truncated) = values
                .get(&anchor)
                .map(|value| {
                    let (text, truncated) = clip(value);
                    (Some(text), truncated)
                })
                .unwrap_or((None, false));
            ContextMerge {
                first_row: merge.first_row,
                first_column: merge.first_column,
                last_row: merge.last_row,
                last_column: merge.last_column,
                anchor_address: cell_address(anchor.0, anchor.1),
                anchor_text,
                anchor_truncated,
            }
        })
        .collect();
    let hidden_rows = sheet
        .hidden_rows
        .into_iter()
        .filter(|row| (range.first_row..=end.0).contains(row))
        .collect();
    let hidden_columns = (range.first_column..=end.1)
        .filter(|column| {
            sheet
                .hidden_columns
                .iter()
                .any(|hidden| (hidden.first..=hidden.last).contains(column))
        })
        .collect();
    ResultContext {
        sheet_name: sheet.sheet_name,
        focus_address: cell_address(target.focus_row, target.focus_column),
        focus_kind: target.focus_kind,
        evidence_addresses: target.evidence_addresses.clone(),
        first_row: range.first_row,
        first_column: range.first_column,
        row_count: range.row_count,
        column_count: range.column_count,
        cells,
        merges,
        hidden_rows,
        hidden_columns,
        layout,
    }
}

pub fn get_result_context(
    target: &ContextTarget,
    range: Option<ContextRange>,
) -> Result<ResultContext, ContextError> {
    let range = range.unwrap_or_else(|| target.initial_range());
    let end = validate_range(target, range)?;
    if target.expected_stamp.is_none() || index::stamp(&target.path) != target.expected_stamp {
        return Err(ContextError::changed());
    }
    let (sheet, merges, cells) = if target.use_index {
        match index_data(target, range, end) {
            Some(data) => data,
            None => direct_data(target, range, end)?,
        }
    } else {
        direct_data(target, range, end)?
    };
    if index::stamp(&target.path) != target.expected_stamp {
        return Err(ContextError::changed());
    }
    Ok(assemble(target, range, end, sheet, merges, cells))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::ColumnRange;

    #[test]
    fn response_limits_styles_to_window_and_preserves_original_match_positions() {
        use crate::extract::{CellLayout, SheetLayout};
        let mut focus = target(12, 2);
        focus.matching_ranges.insert((12, 2), vec![[2, 4]]);
        let range = ContextRange {
            first_row: 10,
            first_column: 2,
            row_count: 5,
            column_count: 5,
        };
        let style = CellStyle {
            bold: true,
            fill: Some("#ddeeff".into()),
            ..Default::default()
        };
        let sheet = SheetMeta {
            layout: SheetLayout {
                styles: vec![CellStyle::default(), style],
                cells: vec![
                    CellLayout {
                        row: 12,
                        column: 2,
                        style: 1,
                    },
                    CellLayout {
                        row: 500,
                        column: 2,
                        style: 1,
                    },
                ],
                rows: vec![
                    RowLayout {
                        row: 12,
                        height: Some(30.0),
                        style: None,
                    },
                    RowLayout {
                        row: 500,
                        height: Some(90.0),
                        style: None,
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        let response = assemble(
            &focus,
            range,
            (14, 6),
            sheet,
            vec![],
            vec![(12, 2, "前文顧客後文".into())],
        );
        assert_eq!(response.layout.cells.len(), 25);
        assert_eq!(response.layout.rows.len(), 1);
        assert!(
            response
                .layout
                .cells
                .iter()
                .find(|cell| cell.row == 12 && cell.column == 2)
                .unwrap()
                .style
                .bold
        );
        assert!(response.layout.cells.iter().all(|cell| cell.row <= 14));
        assert_eq!(response.cells[0].match_ranges, vec![[2, 4]]);
    }

    fn target(row: u32, column: u32) -> ContextTarget {
        ContextTarget {
            path: PathBuf::new(),
            part_key: "xl/worksheets/sheet1.xml".into(),
            sheet_name: "Sheet1".into(),
            focus_row: row,
            focus_column: column,
            focus_kind: "cell",
            evidence_addresses: vec![],
            matching_ranges: BTreeMap::new(),
            expected_stamp: None,
            use_index: false,
        }
    }

    #[test]
    fn window_clamps_to_sheet_corners_and_rejects_large_ranges() {
        let first = target(1, 1);
        assert_eq!(first.initial_range().first_row, 1);
        assert_eq!(first.initial_range().first_column, 1);
        let last = target(LAST_ROW, LAST_COLUMN);
        assert_eq!(last.initial_range().first_row, LAST_ROW - 4);
        assert_eq!(last.initial_range().first_column, LAST_COLUMN - 4);
        assert!(validate_range(
            &first,
            ContextRange {
                first_row: 1,
                first_column: 1,
                row_count: 6,
                column_count: 5
            }
        )
        .is_err());
        assert!(validate_range(
            &last,
            ContextRange {
                first_row: LAST_ROW,
                first_column: LAST_COLUMN,
                row_count: 5,
                column_count: 5
            }
        )
        .is_err());
    }

    #[test]
    fn sparse_cells_merge_anchor_and_hidden_ranges_are_preserved() {
        let focus = target(12, 2);
        let range = ContextRange {
            first_row: 10,
            first_column: 2,
            row_count: 5,
            column_count: 5,
        };
        let sheet = SheetMeta {
            part_key: focus.part_key.clone(),
            sheet_name: "Sheet1".into(),
            merge_ranges: vec![],
            hidden_rows: vec![12],
            hidden_columns: vec![ColumnRange { first: 3, last: 4 }],
            ..SheetMeta::default()
        };
        let merge = CellRange {
            first_row: 12,
            first_column: 1,
            last_row: 12,
            last_column: 4,
        };
        let result = assemble(
            &focus,
            range,
            (14, 6),
            sheet,
            vec![merge],
            vec![
                (12, 1, "顧客番号".into()),
                (12, 2, "顧客".into()),
                (12, 4, "必須".into()),
                (12, 4, "重複".into()),
            ],
        );
        assert_eq!(result.cells.len(), 2);
        assert_eq!(result.cells[0].address, "B12");
        assert_eq!(result.merges[0].anchor_address, "A12");
        assert_eq!(result.merges[0].anchor_text.as_deref(), Some("顧客番号"));
        assert_eq!(result.hidden_rows, vec![12]);
        assert_eq!(result.hidden_columns, vec![3, 4]);
    }
}
