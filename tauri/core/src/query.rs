use crate::extract::Unit;
use crate::fuzzy::{self, Match, Query};
use crate::InputError;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_TERMS_PER_GROUP: usize = 32;
const MAX_TERM_CHARACTERS: usize = 200;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    Unit,
    ExcelRow,
    File,
}

#[derive(Debug)]
pub struct Term {
    pub id: String,
    pub text: String,
    query: Query,
    key: String,
}

#[derive(Debug)]
pub struct Conditions {
    pub scope: Scope,
    all: Vec<Term>,
    any: Vec<Term>,
    not: Vec<Term>,
}

#[derive(Debug)]
pub struct EvidenceCandidate<'a> {
    pub term_id: String,
    pub term: String,
    pub unit: &'a Unit,
    pub matched: Match,
}

#[derive(Debug)]
pub struct GroupMatch<'a> {
    pub source_kind: &'static str,
    pub location: Value,
    pub unit_key: String,
    pub part_key: String,
    pub group_key: String,
    pub row: Option<u32>,
    pub column: Option<u32>,
    pub content_class: String,
    pub anchor: Option<String>,
    pub match_category: &'static str,
    pub evidence: Vec<EvidenceCandidate<'a>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConditions {
    mode: String,
    #[serde(default)]
    all: Vec<String>,
    #[serde(default)]
    any: Vec<String>,
    #[serde(default)]
    not: Vec<String>,
    scope: String,
}

fn invalid(message: &'static str) -> InputError {
    InputError {
        field: "querySpec",
        message,
    }
}

fn terms(values: Vec<String>, prefix: &str) -> Result<Vec<Term>, InputError> {
    if values.len() > 128 {
        return Err(invalid("各条件は32語句以内にしてください。"));
    }
    let mut seen = HashSet::new();
    let mut terms = Vec::new();
    for raw in values {
        let text = raw.trim();
        if text.is_empty() {
            continue;
        }
        if text.chars().count() > MAX_TERM_CHARACTERS {
            return Err(invalid("1語句は200文字以内にしてください。"));
        }
        let key = fuzzy::base(text);
        if seen.insert(key.clone()) {
            terms.push(Term {
                id: format!("{prefix}:{}", terms.len() + 1),
                text: text.to_owned(),
                query: Query::new(text),
                key,
            });
        }
        if terms.len() > MAX_TERMS_PER_GROUP {
            return Err(invalid("各条件は32語句以内にしてください。"));
        }
    }
    Ok(terms)
}

pub fn parse(value: &Value) -> Result<Conditions, InputError> {
    let raw: RawConditions = serde_json::from_value(value.clone())
        .map_err(|_| invalid("高度な検索の条件形式が不正です。"))?;
    if raw.mode != "conditions" {
        return Err(invalid("この検索モードはまだ利用できません。"));
    }
    let scope = match raw.scope.as_str() {
        "unit" => Scope::Unit,
        "excelRow" => Scope::ExcelRow,
        "file" => Scope::File,
        _ => return Err(invalid("判定範囲を選択してください。")),
    };
    let all = terms(raw.all, "all")?;
    let any = terms(raw.any, "any")?;
    let not = terms(raw.not, "not")?;
    if all.is_empty() && any.is_empty() {
        return Err(invalid(
            "「すべて含む」か「いずれか含む」に語句を入力してください。",
        ));
    }
    if all
        .iter()
        .any(|term| not.iter().any(|excluded| excluded.key == term.key))
    {
        return Err(invalid(
            "同じ語句を「すべて含む」と「含まない」に指定できません。",
        ));
    }
    Ok(Conditions {
        scope,
        all,
        any,
        not,
    })
}

fn best_match<'a>(
    units: &[&'a Unit],
    term: &Term,
    fuzzy_search: bool,
    cancel: &AtomicBool,
) -> Option<(&'a Unit, Match)> {
    let mut best: Option<(&'a Unit, Match)> = None;
    for unit in units {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        if let Some(matched) = fuzzy::evaluate_selected(&unit.text, &term.query, fuzzy_search) {
            if best
                .as_ref()
                .is_none_or(|(_, previous)| matched.score > previous.score)
            {
                best = Some((*unit, matched));
            }
        }
    }
    best
}

fn evaluate_group<'a>(
    units: &[&'a Unit],
    spec: &Conditions,
    fuzzy_search: bool,
    cancel: &AtomicBool,
) -> Option<Vec<EvidenceCandidate<'a>>> {
    if !spec.not.is_empty() {
        for unit in units {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let value = fuzzy::base(&unit.text);
            if spec.not.iter().any(|term| value.contains(&term.key)) {
                return None;
            }
        }
    }
    let mut evidence = Vec::new();
    for term in &spec.all {
        let (unit, matched) = best_match(units, term, fuzzy_search, cancel)?;
        evidence.push(EvidenceCandidate {
            term_id: term.id.clone(),
            term: term.text.clone(),
            unit,
            matched,
        });
    }
    let mut any_matched = spec.any.is_empty();
    for term in &spec.any {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        if let Some((unit, matched)) = best_match(units, term, fuzzy_search, cancel) {
            any_matched = true;
            evidence.push(EvidenceCandidate {
                term_id: term.id.clone(),
                term: term.text.clone(),
                unit,
                matched,
            });
        }
    }
    any_matched.then_some(evidence)
}

fn matches_without_fuzzy(units: &[&Unit], spec: &Conditions, cancel: &AtomicBool) -> bool {
    spec.all
        .iter()
        .all(|term| best_match(units, term, false, cancel).is_some())
        && (spec.any.is_empty()
            || spec
                .any
                .iter()
                .any(|term| best_match(units, term, false, cancel).is_some()))
}

pub fn evaluate<'a>(
    units: &[&'a Unit],
    file_type: &str,
    spec: &Conditions,
    fuzzy_search: bool,
    cancel: &AtomicBool,
) -> Vec<GroupMatch<'a>> {
    let mut groups: Vec<Vec<&Unit>> = Vec::new();
    if spec.scope == Scope::File {
        groups.push(units.to_vec());
    } else {
        let mut positions = HashMap::new();
        for (index, &unit) in units.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Vec::new();
            }
            let key = if spec.scope == Scope::ExcelRow
                && matches!(file_type, "xlsx" | "xlsm")
                && unit.source_kind == "cell"
                && unit.meta.row.is_some()
            {
                format!(
                    "row:{}:{}",
                    unit.meta.part_key,
                    unit.meta.row.unwrap_or_default()
                )
            } else {
                format!("unit:{index}")
            };
            let position = *positions.entry(key).or_insert_with(|| {
                groups.push(Vec::new());
                groups.len() - 1
            });
            groups[position].push(unit);
        }
    }
    let mut result = Vec::new();
    for group in groups {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let Some(evidence) = evaluate_group(&group, spec, fuzzy_search, cancel) else {
            continue;
        };
        let match_category = if fuzzy_search && !matches_without_fuzzy(&group, spec, cancel) {
            "fuzzy"
        } else {
            "standard"
        };
        let leader = &evidence[0].unit;
        let is_row = spec.scope == Scope::ExcelRow
            && matches!(file_type, "xlsx" | "xlsm")
            && leader.source_kind == "cell";
        let source_kind = if spec.scope == Scope::File {
            "fileMatch"
        } else if is_row {
            "excelRow"
        } else {
            leader.source_kind
        };
        let location = if spec.scope == Scope::File {
            json!({})
        } else if is_row {
            json!({"sheetName": leader.location["sheetName"], "row": leader.meta.row})
        } else {
            leader.location.clone()
        };
        result.push(GroupMatch {
            source_kind,
            location,
            unit_key: if spec.scope == Scope::File {
                "fileMatch".into()
            } else if is_row {
                leader.meta.group_key.clone()
            } else {
                leader.meta.unit_key.clone()
            },
            part_key: if spec.scope == Scope::File {
                String::new()
            } else {
                leader.meta.part_key.clone()
            },
            group_key: if spec.scope == Scope::File {
                "fileMatch".into()
            } else {
                leader.meta.group_key.clone()
            },
            row: if spec.scope == Scope::File {
                None
            } else {
                leader.meta.row
            },
            column: if spec.scope == Scope::File {
                None
            } else {
                leader.meta.column
            },
            content_class: if spec.scope == Scope::File {
                "body".into()
            } else {
                leader.meta.content_class.clone()
            },
            anchor: if spec.scope == Scope::File {
                None
            } else {
                leader.meta.anchor.clone()
            },
            match_category,
            evidence,
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::UnitMeta;

    fn cell(row: u32, column: u32, text: &str) -> Unit {
        let mut unit = Unit::new(
            "cell",
            json!({"sheetName":"Sheet1", "cellAddress":format!("{column}:{row}")}),
            text.into(),
        );
        unit.meta = UnitMeta {
            unit_key: format!("{row}:{column}"),
            part_key: "sheet1".into(),
            group_key: format!("sheet1#row:{row}"),
            row: Some(row),
            column: Some(column),
            content_class: "body".into(),
            anchor: None,
        };
        unit
    }

    fn spec(scope: &str, all: Vec<&str>, any: Vec<&str>, not: Vec<&str>) -> Conditions {
        parse(&json!({"mode":"conditions", "scope":scope, "all":all, "any":any, "not":not}))
            .unwrap()
    }

    fn run<'a>(
        units: &'a [Unit],
        file_type: &str,
        spec: &Conditions,
        fuzzy_search: bool,
        cancel: &AtomicBool,
    ) -> Vec<GroupMatch<'a>> {
        let references: Vec<&Unit> = units.iter().collect();
        evaluate(&references, file_type, spec, fuzzy_search, cancel)
    }

    #[test]
    fn row_scope_combines_cells_but_not_other_rows_or_unit_scope() {
        let cancel = AtomicBool::new(false);
        let units = vec![
            cell(12, 1, "顧客番号"),
            cell(12, 4, "必須"),
            cell(13, 4, "除外"),
        ];
        let row = spec("excelRow", vec!["顧客", "必須"], vec![], vec!["除外"]);
        assert_eq!(run(&units, "xlsx", &row, false, &cancel).len(), 1);
        assert_eq!(
            run(
                &units,
                "xlsx",
                &spec("unit", vec!["顧客", "必須"], vec![], vec![]),
                false,
                &cancel
            )
            .len(),
            0
        );
        assert_eq!(
            run(
                &units,
                "xlsx",
                &spec("file", vec!["顧客", "必須"], vec![], vec![]),
                false,
                &cancel
            )
            .len(),
            1
        );
        assert_eq!(
            run(
                &units,
                "xlsx",
                &spec("file", vec!["顧客", "必須"], vec![], vec!["除外"]),
                false,
                &cancel
            )
            .len(),
            0
        );
    }

    #[test]
    fn normalization_and_exclusion_rules() {
        let cancel = AtomicBool::new(false);
        let query = spec(
            "excelRow",
            vec![" Customer ID ", "customer id"],
            vec!["AND OR"],
            vec!["blocked"],
        );
        assert_eq!(query.all.len(), 1);
        assert_eq!(query.all[0].text, "Customer ID");
        assert!(
            parse(&json!({"mode":"conditions", "scope":"unit", "all":["A"], "not":["a"]})).is_err()
        );
        assert!(parse(&json!({"mode":"conditions", "scope":"unit", "not":["A"]})).is_err());
        let units = vec![
            cell(12, 1, "顧客"),
            cell(12, 2, "必須"),
            cell(12, 3, "BLOCKED"),
        ];
        assert!(run(
            &units,
            "xlsx",
            &spec("excelRow", vec!["顧客"], vec![], vec!["blocked"]),
            false,
            &cancel
        )
        .is_empty());
    }
}
