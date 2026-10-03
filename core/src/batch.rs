use crate::{fuzzy, InputError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug)]
pub struct Batch {
    pub terms: Vec<Term>,
    pub identifier: bool,
}

#[derive(Debug)]
pub struct Term {
    pub id: String,
    pub text: String,
    query: fuzzy::Query,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawBatch {
    mode: String,
    terms: Vec<String>,
    #[serde(default = "identifier_mode")]
    match_mode: String,
}

fn identifier_mode() -> String {
    "identifier".into()
}
fn invalid(message: &'static str) -> InputError {
    InputError {
        field: "querySpec",
        message,
    }
}

pub fn parse(value: &Value) -> Result<Batch, InputError> {
    let raw: RawBatch = serde_json::from_value(value.clone())
        .map_err(|_| invalid("一括検索の条件形式が不正です。"))?;
    if raw.mode != "batch" || !matches!(raw.match_mode.as_str(), "identifier" | "text") {
        return Err(invalid("一括検索の照合方法を選択してください。"));
    }
    if raw.terms.len() > 4096 {
        return Err(invalid("一括検索の入力行数が多すぎます。"));
    }
    let identifier = raw.match_mode == "identifier";
    let mut seen = HashSet::new();
    let mut terms = Vec::new();
    for raw in raw.terms {
        let text = raw.trim();
        if text.is_empty() {
            continue;
        }
        if text.chars().count() > 200 {
            return Err(invalid("1検索語は200文字以内にしてください。"));
        }
        if identifier && !valid_identifier(text) {
            return Err(invalid("識別子一致は英字・数字・アンダースコア（先頭は英字またはアンダースコア）を使います。日本語・空白・記号を含む行は「通常の文字列検索」に切り替えてください。"));
        }
        if seen.insert(fuzzy::base(text)) {
            terms.push(Term {
                id: format!("batch:{}", terms.len() + 1),
                text: text.into(),
                query: fuzzy::Query::new(text),
            });
        }
        if terms.len() > 256 {
            return Err(invalid("一括検索は重複を除いて256語以内にしてください。"));
        }
    }
    if terms.is_empty() {
        return Err(invalid("一括検索の検索語を1行ずつ入力してください。"));
    }
    Ok(Batch { terms, identifier })
}

pub fn valid_identifier(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl Batch {
    pub fn matched(&self, term: &Term, text: &str, fuzzy_search: bool) -> Option<fuzzy::Match> {
        self.matched_prepared(term, &fuzzy::PreparedText::new(text), fuzzy_search)
    }

    pub(crate) fn matched_prepared(
        &self,
        term: &Term,
        text: &fuzzy::PreparedText<'_>,
        fuzzy_search: bool,
    ) -> Option<fuzzy::Match> {
        if !self.identifier {
            return text.evaluate_selected(&term.query, fuzzy_search);
        }
        text.identifier(&term.query.base).map(|range| fuzzy::Match {
            kind: "identifier",
            category: "standard",
            score: 90,
            range,
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TermSummary {
    pub term_id: String,
    pub term: String,
    pub file_count: usize,
    pub hit_count: usize,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn identifiers_respect_boundaries_and_original_unicode_positions() {
        let spec = parse(&json!({"mode":"batch", "terms":["", "TAB_01", "tab_01"]})).unwrap();
        assert_eq!(spec.terms.len(), 1);
        let term = &spec.terms[0];
        assert!(spec.matched(term, "TAB_010", true).is_none());
        assert!(spec.matched(term, "顧客TAB_01", true).is_none());
        assert!(spec.matched(term, "ＴＡＢ_01", true).is_none());
        assert_eq!(
            spec.matched(term, "👩‍💻 tab_01 / TAB_010", true)
                .unwrap()
                .range,
            [4, 10]
        );
        assert!(parse(&json!({"mode":"batch", "terms":["顧客"]})).is_err());
    }

    #[test]
    fn text_terms_deduplicate_normalization_and_preserve_first_display() {
        let spec = parse(&json!({"mode":"batch", "matchMode":"text", "terms":[
            " CAFÉ ", "cafe\u{301}", "顧客番号", "", "顧客番号"
        ]}))
        .unwrap();
        assert_eq!(spec.terms.len(), 2);
        assert_eq!(spec.terms[0].text, "CAFÉ");
        assert_eq!(spec.terms[1].text, "顧客番号");
        assert!(spec
            .matched(&spec.terms[1], "顧客番号の説明", false)
            .is_some());
    }
}
