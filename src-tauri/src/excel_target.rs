//! Resolve only positions supplied by the current search, without starting Excel.
use docs_search_core::SearchHit;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExcelTarget {
    pub file_path: String,
    pub sheet_name: String,
    pub cell_address: String,
}

fn valid_address(address: &str) -> bool {
    let split = address.bytes().take_while(u8::is_ascii_uppercase).count();
    let (letters, digits) = address.split_at(split);
    if letters.is_empty()
        || letters.len() > 3
        || digits.is_empty()
        || digits.starts_with('0')
        || !digits.bytes().all(|c| c.is_ascii_digit())
    {
        return false;
    }
    let column = letters
        .bytes()
        .fold(0u32, |n, c| n * 26 + u32::from(c - b'A' + 1));
    column <= 16_384
        && digits
            .parse::<u32>()
            .is_ok_and(|row| (1..=1_048_576).contains(&row))
}

fn position<'a>(kind: &str, location: &'a Value) -> Option<(&'a str, &'a str)> {
    let sheet = location.get("sheetName")?.as_str()?;
    let address = match kind {
        "cell" | "formula" | "excelComment" => location.get("cellAddress")?.as_str()?,
        "shape" => location.get("anchor")?.as_str()?,
        _ => return None,
    };
    (!sheet.is_empty() && valid_address(address)).then_some((sheet, address))
}

pub fn resolve(hit: &SearchHit, evidence_index: Option<usize>) -> Result<ExcelTarget, String> {
    if !matches!(hit.file_type.as_str(), "xlsx" | "xlsm") {
        return Err("この形式はExcelのセル移動に対応していません。".into());
    }
    let (kind, location) = if hit.evidence.is_empty() {
        if evidence_index.is_some() {
            return Err("指定された根拠がありません。".into());
        }
        (hit.source_kind.as_str(), &hit.location)
    } else {
        let source = evidence_index
            .and_then(|index| hit.evidence.get(index))
            .ok_or("移動する根拠を選んでください。")?;
        (source.source_kind.as_str(), &source.location)
    };
    let (sheet, address) = position(kind, location)
        .ok_or("シート名とセル番地を特定できません。表示された位置を確認し、ファイル名から開いてください。")?;
    Ok(ExcelTarget {
        file_path: hit.file_path.clone(),
        sheet_name: sheet.into(),
        cell_address: address.into(),
    })
}
