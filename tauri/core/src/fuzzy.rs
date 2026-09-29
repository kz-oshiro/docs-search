use std::collections::HashSet;
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug)]
pub struct Match {
    pub kind: &'static str,
    pub score: u8,
    pub range: [usize; 2],
}

#[derive(Clone, Debug)]
pub struct Query {
    pub raw: String,
    pub base: String,
    pub width: String,
    pub kana: String,
    pub loose: String,
    pub tokens: Vec<String>,
    pub typo: Option<String>,
    numeric: bool,
    allow_loose: bool,
}

pub fn base(text: &str) -> String {
    text.nfc().case_fold().collect()
}

fn width(text: &str) -> String {
    text.nfkc().case_fold().collect()
}

fn kana(text: &str) -> String {
    width(text)
        .chars()
        .map(|c| match c {
            '\u{3041}'..='\u{3096}' => char::from_u32(c as u32 + 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

fn separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, '_' | '-' | '.' | '/')
}

fn loose(text: &str) -> String {
    kana(text).chars().filter(|c| !separator(*c)).collect()
}

pub fn loose_key(text: &str) -> String {
    loose(text)
}

fn split_identifier(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut start = 0;
    for i in 0..=chars.len() {
        let boundary = i == chars.len()
            || separator(chars[i])
            || !chars[i].is_alphanumeric()
            || (i > start
                && ((chars[i - 1].is_ascii_lowercase() && chars[i].is_ascii_uppercase())
                    || (chars[i - 1].is_ascii_digit() != chars[i].is_ascii_digit()
                        && chars[i].is_ascii())
                    || (i + 1 < chars.len()
                        && chars[i - 1].is_ascii_uppercase()
                        && chars[i].is_ascii_uppercase()
                        && chars[i + 1].is_ascii_lowercase())));
        if boundary {
            if start < i {
                words.push(width(&chars[start..i].iter().collect::<String>()));
            }
            start = if i < chars.len() && (separator(chars[i]) || !chars[i].is_alphanumeric()) {
                i + 1
            } else {
                i
            };
        }
    }
    words
}

pub fn tokens(text: &str) -> Vec<String> {
    split_identifier(text)
        .into_iter()
        .filter(|word| !word.is_empty())
        .collect()
}

pub fn grams(text: &str, size: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < size {
        return Vec::new();
    }
    chars
        .windows(size)
        .map(|part| part.iter().collect())
        .collect::<HashSet<String>>()
        .into_iter()
        .collect()
}

impl Query {
    pub fn new(raw: &str) -> Self {
        let raw = raw.trim().to_owned();
        let tokens = tokens(&raw);
        let typo = if tokens.len() == 1
            && tokens[0].chars().count() >= 6
            && tokens[0].chars().all(|c| c.is_ascii_alphabetic())
        {
            Some(tokens[0].clone())
        } else {
            None
        };
        let numeric = raw.chars().all(|c| c.is_ascii_digit() || separator(c));
        let kana = kana(&raw);
        let loose = if numeric { kana.clone() } else { loose(&raw) };
        let allow_loose = !numeric && loose.chars().count() >= 3;
        Self {
            base: base(&raw),
            width: width(&raw),
            loose,
            kana,
            tokens,
            typo,
            numeric,
            allow_loose,
            raw,
        }
    }
}

pub fn could_match(text: &str, query: &Query) -> bool {
    if query.numeric {
        return kana(text).contains(&query.loose);
    }
    if (if query.allow_loose {
        loose(text)
    } else {
        kana(text)
    })
    .contains(if query.allow_loose {
        &query.loose
    } else {
        &query.kana
    }) {
        return true;
    }
    let Some(typo) = &query.typo else {
        return false;
    };
    let limit = if typo.len() >= 11 { 2 } else { 1 };
    tokens(text).into_iter().any(|token| {
        token.starts_with(typo.chars().next().unwrap_or_default())
            && token.len().abs_diff(typo.len()) <= limit
    })
}

fn mapped(text: &str, convert: fn(&str) -> String) -> (String, Vec<[usize; 2]>) {
    let mut value = String::new();
    let mut map = Vec::new();
    let mut position = 0;
    for grapheme in text.graphemes(true) {
        let end = position + grapheme.chars().count();
        let part = convert(grapheme);
        map.extend(std::iter::repeat_n([position, end], part.len()));
        value.push_str(&part);
        position = end;
    }
    (value, map)
}

fn found(text: &str, needle: &str, convert: fn(&str) -> String) -> Option<[usize; 2]> {
    if needle.is_empty() {
        return None;
    }
    let (haystack, map) = mapped(text, convert);
    let offset = haystack.find(needle)?;
    Some([map[offset][0], map[offset + needle.len() - 1][1]])
}

fn found_whole(text: &str, needle: &str, convert: fn(&str) -> String) -> Option<[usize; 2]> {
    if needle.is_empty() {
        return None;
    }
    let (haystack, map) = mapped(text, convert);
    haystack
        .match_indices(needle)
        .map(|(offset, _)| [map[offset][0], map[offset + needle.len() - 1][1]])
        .find(|range| whole(text, *range))
}

fn whole(text: &str, range: [usize; 2]) -> bool {
    let chars: Vec<char> = text.chars().collect();
    (range[0] == 0 || !chars[range[0] - 1].is_alphanumeric())
        && (range[1] == chars.len() || !chars[range[1]].is_alphanumeric())
}

fn distance(a: &str, b: &str, limit: usize) -> Option<usize> {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > limit {
        return None;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ac) in a.iter().enumerate() {
        let mut next = vec![i + 1; b.len() + 1];
        for (j, bc) in b.iter().enumerate() {
            next[j + 1] = (prev[j + 1] + 1)
                .min(next[j] + 1)
                .min(prev[j] + usize::from(ac != bc));
        }
        if *next.iter().min().unwrap_or(&usize::MAX) > limit {
            return None;
        }
        prev = next;
    }
    (prev[b.len()] <= limit).then_some(prev[b.len()])
}

pub fn evaluate(text: &str, query: &Query) -> Option<Match> {
    let options: [(fn(&str) -> String, &str, &str, u8); 5] = [
        (str::to_owned, &query.raw, "exact", 100),
        (base, &query.base, "caseFolded", 98),
        (width, &query.width, "normalized", 95),
        (kana, &query.kana, "kanaVariant", 88),
        (loose, &query.loose, "identifier", 90),
    ];
    for (convert, needle, kind, score) in options {
        if kind != "identifier" || query.allow_loose {
            if let Some(range) = found_whole(text, needle, convert) {
                let (kind, score) = if kind == "identifier" {
                    if query.raw.chars().any(char::is_whitespace)
                        || text.chars().any(char::is_whitespace)
                        || query.raw.chars().any(|c| matches!(c, '.' | '/'))
                        || text.chars().any(|c| matches!(c, '.' | '/'))
                    {
                        ("separatorVariant", 92)
                    } else {
                        ("identifier", score)
                    }
                } else {
                    (kind, score)
                };
                return Some(Match { kind, score, range });
            }
        }
    }
    for token in tokens_with_ranges(text) {
        if let Some(query_token) = query.tokens.first() {
            if token.0 == *query_token && query.tokens.len() == 1 {
                return Some(Match {
                    kind: "identifier",
                    score: 90,
                    range: token.1,
                });
            }
            if token.0.starts_with(query_token)
                && query.tokens.len() == 1
                && query_token.chars().count() >= 3
            {
                return Some(Match {
                    kind: "prefix",
                    score: 80,
                    range: token.1,
                });
            }
        }
    }
    if let Some(range) = found(
        text,
        if query.allow_loose {
            &query.loose
        } else {
            &query.kana
        },
        if query.allow_loose { loose } else { kana },
    ) {
        return Some(Match {
            kind: "substring",
            score: 70,
            range,
        });
    }
    if let Some(typo) = &query.typo {
        let limit = if typo.len() >= 11 { 2 } else { 1 };
        for (token, range) in tokens_with_ranges(text) {
            if token.chars().all(|c| c.is_ascii_alphabetic())
                && token.starts_with(typo.chars().next().unwrap_or_default())
                && distance(typo, &token, limit).is_some_and(|d| d > 0 && d * 5 <= typo.len())
            {
                return Some(Match {
                    kind: "editDistance",
                    score: 50,
                    range,
                });
            }
        }
    }
    None
}

fn tokens_with_ranges(text: &str) -> Vec<(String, [usize; 2])> {
    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;
    for i in 0..=chars.len() {
        let boundary = i == chars.len()
            || !chars[i].is_alphanumeric()
            || (i > start && chars[i - 1].is_ascii_lowercase() && chars[i].is_ascii_uppercase())
            || (i > start
                && i + 1 < chars.len()
                && chars[i - 1].is_ascii_uppercase()
                && chars[i].is_ascii_uppercase()
                && chars[i + 1].is_ascii_lowercase());
        if boundary {
            if start < i {
                result.push((
                    width(&chars[start..i].iter().collect::<String>()),
                    [start, i],
                ));
            }
            start = if i < chars.len() && !chars[i].is_alphanumeric() {
                i + 1
            } else {
                i
            };
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{evaluate, tokens, Query};

    #[test]
    fn identifier_variants_and_width_keep_original_ranges() {
        let query = Query::new("customerId");
        for text in [
            "customerId",
            "CustomerId",
            "CUSTOMER_ID",
            "customer_id",
            "customer-id",
            "customer id",
        ] {
            let result = evaluate(text, &query).expect(text);
            assert_eq!(result.range, [0, text.chars().count()]);
        }
        assert_eq!(
            tokens("CustomerID"),
            vec![String::from("customer"), String::from("id")]
        );
        assert_eq!(
            evaluate("顧客ＩＤ", &Query::new("顧客ID")).unwrap().kind,
            "normalized"
        );
        assert_eq!(
            evaluate("顧客 ID", &Query::new("顧客ID")).unwrap().kind,
            "separatorVariant"
        );
    }

    #[test]
    fn typo_is_below_exact_and_excludes_short_or_numeric_terms() {
        assert_eq!(
            evaluate("customer", &Query::new("customer")).unwrap().score,
            100
        );
        assert_eq!(
            evaluate("custmer", &Query::new("customer")).unwrap().kind,
            "editDistance"
        );
        assert!(evaluate("NO", &Query::new("ID")).is_none());
        assert!(evaluate("1/2", &Query::new("12")).is_none());
        assert!(evaluate("10", &Query::new("1.0")).is_none());
    }
}
