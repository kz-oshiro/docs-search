use crate::{fuzzy, SearchHit};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankingEvidence {
    pub quality: u8,
    pub matched_terms: usize,
    pub same_row_terms: usize,
    pub body_evidence: usize,
    pub distinct_evidence: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRanking {
    pub file_path: String,
    pub evidence: RankingEvidence,
    pub reasons: Vec<String>,
    pub result_ids: Vec<usize>,
}

pub fn summarize(hits: &[SearchHit], same_row_terms: usize) -> Option<FileRanking> {
    let first = hits.first()?;
    let mut terms: HashMap<String, u8> = HashMap::new();
    let mut rows: HashMap<(String, u32), HashSet<String>> = HashMap::new();
    let mut body = HashSet::new();
    let mut places = HashSet::new();
    for hit in hits {
        let mut record = |term: String,
                          score: u8,
                          part: &str,
                          unit: &str,
                          row: Option<u32>,
                          class: &str,
                          kind: &str| {
            terms
                .entry(term.clone())
                .and_modify(|value| *value = (*value).max(score))
                .or_insert(score);
            let place = (part.to_owned(), unit.to_owned());
            places.insert(place.clone());
            if class == "body" {
                body.insert(place);
            }
            if matches!(kind, "cell" | "formula") {
                if let Some(row) = row {
                    rows.entry((part.into(), row)).or_default().insert(term);
                }
            }
        };
        if hit.evidence.is_empty() {
            record(
                hit.term
                    .as_deref()
                    .map(fuzzy::base)
                    .unwrap_or_else(|| "query".into()),
                hit.score,
                &hit.part_key,
                &hit.unit_key,
                hit.row,
                &hit.content_class,
                &hit.source_kind,
            );
        } else {
            for item in &hit.evidence {
                record(
                    fuzzy::base(&item.term),
                    item.score,
                    &item.part_key,
                    &item.unit_key,
                    item.row,
                    &item.content_class,
                    &item.source_kind,
                );
            }
        }
    }
    let evidence = RankingEvidence {
        quality: terms.values().copied().min().unwrap_or(0),
        matched_terms: terms.len(),
        same_row_terms: rows
            .values()
            .map(HashSet::len)
            .filter(|count| *count > 1)
            .max()
            .unwrap_or(0)
            .max(same_row_terms),
        body_evidence: body.len().min(5),
        distinct_evidence: places.len().min(5),
    };
    let mut ordered: Vec<_> = hits.iter().collect();
    ordered.sort_by(|a, b| {
        a.document_order
            .cmp(&b.document_order)
            .then_with(|| a.term_id.cmp(&b.term_id))
            .then_with(|| a.unit_key.cmp(&b.unit_key))
            .then_with(|| a.result_id.cmp(&b.result_id))
    });
    let mut reasons = vec![format!("{}語に一致", evidence.matched_terms)];
    if evidence.same_row_terms > 1 {
        reasons.push(format!("同じ行で{}語に一致", evidence.same_row_terms));
    }
    if evidence.body_evidence > 0 {
        reasons.push("本文に根拠あり".into());
    }
    Some(FileRanking {
        file_path: first.file_path.clone(),
        evidence,
        reasons,
        result_ids: ordered.iter().map(|hit| hit.result_id).collect(),
    })
}

pub fn compare(a: &FileRanking, b: &FileRanking) -> Ordering {
    let key = |file: &FileRanking| {
        let e = &file.evidence;
        (
            e.quality,
            e.matched_terms,
            e.same_row_terms,
            e.body_evidence,
            e.distinct_evidence,
        )
    };
    key(b)
        .cmp(&key(a))
        .then_with(|| fuzzy::base(&a.file_path).cmp(&fuzzy::base(&b.file_path)))
        .then_with(|| a.file_path.cmp(&b.file_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparison_obeys_each_priority_before_later_evidence() {
        let file = |quality, terms, row, body, count, path: &str| FileRanking {
            file_path: path.into(),
            evidence: RankingEvidence {
                quality,
                matched_terms: terms,
                same_row_terms: row,
                body_evidence: body,
                distinct_evidence: count,
            },
            reasons: vec![],
            result_ids: vec![],
        };
        let base = file(90, 2, 2, 1, 1, "a");
        for higher in [
            file(98, 1, 1, 0, 0, "z"),
            file(90, 3, 1, 0, 0, "z"),
            file(90, 2, 3, 0, 0, "z"),
            file(90, 2, 2, 2, 0, "z"),
            file(90, 2, 2, 1, 2, "z"),
        ] {
            assert_eq!(compare(&higher, &base), Ordering::Less);
        }
        assert_eq!(compare(&base, &file(90, 2, 2, 1, 1, "b")), Ordering::Less);
    }
}
