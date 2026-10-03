use std::cell::OnceCell;
use std::collections::HashSet;
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug)]
pub struct Match {
    pub kind: &'static str,
    pub category: &'static str,
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
    // The evaluator also accepts a single identifier token or token prefix,
    // even when punctuation in the original query is absent from the source.
    if query.tokens.len() == 1 {
        let token = &query.tokens[0];
        if tokens_with_ranges(text).into_iter().any(|(candidate, _)| {
            candidate == *token || (token.chars().count() >= 3 && candidate.starts_with(token))
        }) {
            return true;
        }
    }
    if query.numeric {
        return false;
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

fn whole(chars: &[char], range: [usize; 2]) -> bool {
    (range[0] == 0 || !chars[range[0] - 1].is_alphanumeric())
        && (range[1] == chars.len() || !chars[range[1]].is_alphanumeric())
}

#[derive(Clone, Copy)]
enum Form {
    Raw,
    Base,
    Width,
    Kana,
    Loose,
}

impl Form {
    fn convert(self, text: &str) -> String {
        match self {
            Self::Raw => text.to_owned(),
            Self::Base => base(text),
            Self::Width => width(text),
            Self::Kana => kana(text),
            Self::Loose => loose(text),
        }
    }
}

struct MappingRun {
    byte_end: usize,
    source_start: usize,
    source_end: usize,
    linear: bool,
}

struct MappedText {
    value: String,
    runs: OnceCell<Vec<MappingRun>>,
    form: Form,
}

impl MappedText {
    fn new(text: &str, form: Form) -> Self {
        let value = if matches!(form, Form::Raw) {
            text.to_owned()
        } else if text.is_ascii() {
            // ASCII has no normalization or kana expansion. Avoid an allocation
            // for every single-character grapheme while keeping CRLF mapping.
            let folded = text.to_ascii_lowercase();
            if matches!(form, Form::Loose) {
                folded.chars().filter(|c| !separator(*c)).collect()
            } else {
                folded
            }
        } else {
            let mut value = String::with_capacity(text.len());
            for grapheme in text.graphemes(true) {
                value.push_str(&form.convert(grapheme));
            }
            value
        };
        Self {
            value,
            runs: OnceCell::new(),
            form,
        }
    }

    fn runs(&self, text: &str) -> &[MappingRun] {
        self.runs.get_or_init(|| {
            let mut runs: Vec<MappingRun> = Vec::new();
            let mut position = 0;
            let mut byte_end = 0;
            let ascii = text.is_ascii();
            for grapheme in text.graphemes(true) {
                let end = position + grapheme.chars().count();
                let length = if matches!(self.form, Form::Raw) {
                    grapheme.len()
                } else if ascii {
                    if matches!(self.form, Form::Loose) {
                        grapheme.chars().filter(|c| !separator(*c)).count()
                    } else {
                        grapheme.len()
                    }
                } else {
                    self.form.convert(grapheme).len()
                };
                byte_end += length;
                if length != 0 {
                    // Coalesce one-byte, one-scalar mappings. Unicode expansions and
                    // multi-scalar graphemes retain the original full source range.
                    let linear = length == 1 && end == position + 1;
                    if linear
                        && runs
                            .last()
                            .is_some_and(|run| run.linear && run.source_end == position)
                    {
                        let previous = runs.last_mut().unwrap();
                        previous.byte_end = byte_end;
                        previous.source_end = end;
                    } else {
                        runs.push(MappingRun {
                            byte_end,
                            source_start: position,
                            source_end: end,
                            linear,
                        });
                    }
                }
                position = end;
            }
            runs
        })
    }

    fn source_at(runs: &[MappingRun], byte: usize) -> [usize; 2] {
        let index = runs.partition_point(|run| run.byte_end <= byte);
        let run = &runs[index];
        if run.linear {
            let start = if index == 0 {
                0
            } else {
                runs[index - 1].byte_end
            };
            let position = run.source_start + byte - start;
            [position, position + 1]
        } else {
            [run.source_start, run.source_end]
        }
    }

    fn range(&self, text: &str, byte: usize, length: usize) -> [usize; 2] {
        let runs = self.runs(text);
        [
            Self::source_at(runs, byte)[0],
            Self::source_at(runs, byte + length - 1)[1],
        ]
    }
}

/// Lazy views live for one source unit, shared by all terms evaluating it.
pub(crate) struct PreparedText<'a> {
    text: &'a str,
    chars: OnceCell<Vec<char>>,
    raw: OnceCell<MappedText>,
    base: OnceCell<MappedText>,
    width: OnceCell<MappedText>,
    kana: OnceCell<MappedText>,
    loose: OnceCell<MappedText>,
    tokens: OnceCell<Vec<(String, [usize; 2])>>,
}

impl<'a> PreparedText<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        Self {
            text,
            chars: OnceCell::new(),
            raw: OnceCell::new(),
            base: OnceCell::new(),
            width: OnceCell::new(),
            kana: OnceCell::new(),
            loose: OnceCell::new(),
            tokens: OnceCell::new(),
        }
    }

    fn chars(&self) -> &[char] {
        self.chars.get_or_init(|| self.text.chars().collect())
    }

    fn mapped(&self, form: Form) -> &MappedText {
        let slot = match form {
            Form::Raw => &self.raw,
            Form::Base => &self.base,
            Form::Width => &self.width,
            Form::Kana => &self.kana,
            Form::Loose => &self.loose,
        };
        slot.get_or_init(|| MappedText::new(self.text, form))
    }

    fn found(&self, needle: &str, form: Form) -> Option<[usize; 2]> {
        if needle.is_empty() {
            return None;
        }
        let mapped = self.mapped(form);
        let offset = mapped.value.find(needle)?;
        Some(mapped.range(self.text, offset, needle.len()))
    }

    fn found_whole(&self, needle: &str, form: Form) -> Option<[usize; 2]> {
        if needle.is_empty() {
            return None;
        }
        let mapped = self.mapped(form);
        mapped
            .value
            .match_indices(needle)
            .map(|(offset, _)| mapped.range(self.text, offset, needle.len()))
            .find(|range| whole(self.chars(), *range))
    }

    fn tokens(&self) -> &[(String, [usize; 2])] {
        self.tokens.get_or_init(|| tokens_from_chars(self.chars()))
    }

    pub(crate) fn identifier(&self, needle: &str) -> Option<[usize; 2]> {
        if needle.is_empty() {
            return None;
        }
        let mapped = self.mapped(Form::Base);
        let identifier_char = |c: char| c.is_alphanumeric() || c == '_';
        mapped.value.match_indices(needle).find_map(|(start, _)| {
            let end = start + needle.len();
            if mapped.value[..start]
                .chars()
                .next_back()
                .is_some_and(identifier_char)
                || mapped.value[end..]
                    .chars()
                    .next()
                    .is_some_and(identifier_char)
            {
                None
            } else {
                Some(mapped.range(self.text, start, needle.len()))
            }
        })
    }
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

pub fn evaluate_selected(text: &str, query: &Query, fuzzy_search: bool) -> Option<Match> {
    PreparedText::new(text).evaluate_selected(query, fuzzy_search)
}

impl PreparedText<'_> {
    pub(crate) fn evaluate_selected(&self, query: &Query, fuzzy_search: bool) -> Option<Match> {
        if !fuzzy_search {
            return self.evaluate_standard(query);
        }
        self.evaluate(query).map(|mut matched| {
            // Keep the strongest fuzzy-mode reason, but show whether normal search also finds this unit.
            if self.evaluate_standard(query).is_some() {
                matched.category = "standard";
            }
            matched
        })
    }

    fn evaluate_standard(&self, query: &Query) -> Option<Match> {
        if let Some(range) = self.found_whole(&query.raw, Form::Raw) {
            return Some(Match {
                kind: "exact",
                category: "standard",
                score: 100,
                range,
            });
        }
        if let Some(range) = self.found_whole(&query.base, Form::Base) {
            return Some(Match {
                kind: "caseFolded",
                category: "standard",
                score: 98,
                range,
            });
        }
        self.found(&query.base, Form::Base).map(|range| Match {
            kind: "substring",
            category: "standard",
            score: 70,
            range,
        })
    }

    fn evaluate(&self, query: &Query) -> Option<Match> {
        let options: [(Form, &str, &str, u8); 5] = [
            (Form::Raw, &query.raw, "exact", 100),
            (Form::Base, &query.base, "caseFolded", 98),
            (Form::Width, &query.width, "normalized", 95),
            (Form::Kana, &query.kana, "kanaVariant", 88),
            (Form::Loose, &query.loose, "identifier", 90),
        ];
        for (form, needle, kind, score) in options {
            if kind != "identifier" || query.allow_loose {
                if let Some(range) = self.found_whole(needle, form) {
                    let (kind, score) = if kind == "identifier" {
                        if query.raw.chars().any(char::is_whitespace)
                            || self.text.chars().any(char::is_whitespace)
                            || query.raw.chars().any(|c| matches!(c, '.' | '/'))
                            || self.text.chars().any(|c| matches!(c, '.' | '/'))
                        {
                            ("separatorVariant", 92)
                        } else {
                            ("identifier", score)
                        }
                    } else {
                        (kind, score)
                    };
                    return Some(Match {
                        kind,
                        category: "fuzzy",
                        score,
                        range,
                    });
                }
            }
        }
        for token in self.tokens() {
            if let Some(query_token) = query.tokens.first() {
                if token.0 == *query_token && query.tokens.len() == 1 {
                    return Some(Match {
                        kind: "identifier",
                        category: "fuzzy",
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
                        category: "fuzzy",
                        score: 80,
                        range: token.1,
                    });
                }
            }
        }
        if let Some(range) = self.found(
            if query.allow_loose {
                &query.loose
            } else {
                &query.kana
            },
            if query.allow_loose {
                Form::Loose
            } else {
                Form::Kana
            },
        ) {
            return Some(Match {
                kind: "substring",
                category: "fuzzy",
                score: 70,
                range,
            });
        }
        if let Some(typo) = &query.typo {
            let limit = if typo.len() >= 11 { 2 } else { 1 };
            for (token, range) in self.tokens() {
                if token.chars().all(|c| c.is_ascii_alphabetic())
                    && token.starts_with(typo.chars().next().unwrap_or_default())
                    && distance(typo, token, limit).is_some_and(|d| d > 0 && d * 5 <= typo.len())
                {
                    return Some(Match {
                        kind: "editDistance",
                        category: "fuzzy",
                        score: 50,
                        range: *range,
                    });
                }
            }
        }
        None
    }
}

#[cfg(test)]
pub fn evaluate(text: &str, query: &Query) -> Option<Match> {
    PreparedText::new(text).evaluate(query)
}

fn tokens_with_ranges(text: &str) -> Vec<(String, [usize; 2])> {
    tokens_from_chars(&text.chars().collect::<Vec<_>>())
}

fn tokens_from_chars(chars: &[char]) -> Vec<(String, [usize; 2])> {
    let mut result = Vec::new();
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
    use super::{could_match, evaluate, tokens, Query};

    #[test]
    fn repeated_substrings_keep_whole_match_priority_and_original_positions() {
        let repeated = "a".repeat(16_384);
        let query = Query::new("a");
        let text = format!("{repeated} a");
        for fuzzy in [false, true] {
            let matched = super::evaluate_selected(&text, &query, fuzzy).unwrap();
            assert_eq!(
                (matched.kind, matched.category, matched.score),
                ("exact", "standard", 100)
            );
            assert_eq!(matched.range, [16_385, 16_386]);
            let substring = super::evaluate_selected(&repeated, &query, fuzzy).unwrap();
            assert_eq!(
                (substring.kind, substring.category, substring.score),
                ("substring", "standard", 70)
            );
            assert_eq!(substring.range, [0, 1]);
        }
    }

    #[test]
    fn shared_views_preserve_expansions_graphemes_and_removed_separators() {
        let prepared = super::PreparedText::new("👩‍💻 Straße cafe\u{301} / customer-id");
        for (query, range, kind, category) in [
            ("STRASSE", [4, 10], "caseFolded", "standard"),
            ("café", [11, 16], "caseFolded", "standard"),
            ("customerId", [19, 30], "separatorVariant", "fuzzy"),
            ("👩", [0, 3], "exact", "standard"),
        ] {
            let matched = prepared
                .evaluate_selected(&Query::new(query), true)
                .unwrap();
            assert_eq!(matched.range, range, "{query}");
            assert_eq!(
                (matched.kind, matched.category),
                (kind, category),
                "{query}"
            );
        }
        let crlf = super::evaluate_selected("p x\r\ny q", &Query::new("x\r\ny"), false).unwrap();
        assert_eq!(crlf.range, [2, 6]);
        let ascii = super::PreparedText::new("p CUSTOMER-ID q");
        let identifier = ascii
            .evaluate_selected(&Query::new("customerId"), true)
            .unwrap();
        assert_eq!(identifier.range, [2, 13]);
        assert_eq!(
            (identifier.kind, identifier.category),
            ("separatorVariant", "fuzzy")
        );
        let folded = ascii
            .evaluate_selected(&Query::new("customer-id"), false)
            .unwrap();
        assert_eq!(folded.range, [2, 13]);
        assert_eq!((folded.kind, folded.category), ("caseFolded", "standard"));
    }

    #[test]
    fn token_matches_are_not_lost_when_query_punctuation_is_absent() {
        for (text, raw) in [("a_", "a!"), ("alphabet_", "alpha!")] {
            let query = Query::new(raw);
            assert!(evaluate(text, &query).is_some());
            assert!(could_match(text, &query));
        }
    }

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
