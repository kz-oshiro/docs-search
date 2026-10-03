mod support;
use serde_json::{json, Value};
use support::*;
fn place(file: &str, kind: &str, sheet: Value, row: Value, address: Value, line: Value) -> Value {
    json!([file, kind, sheet, row, address, line])
}
fn row(n: usize, sheet: &str) -> Value {
    place(
        "conditions.xlsx",
        "excelRow",
        json!(sheet),
        json!(n),
        json!(null),
        json!(null),
    )
}
fn places(hits: &[Value]) -> Vec<String> {
    let mut values: Vec<_> = hits
        .iter()
        .map(|h| {
            canonical(&place(
                &filename(&h["filePath"]),
                h["sourceKind"].as_str().unwrap(),
                h["location"]["sheetName"].clone(),
                h["location"]["row"].clone(),
                h["location"]["cellAddress"].clone(),
                h["location"]["lineNumber"].clone(),
            ))
        })
        .collect();
    values.sort();
    values
}
fn accepted(
    cli: &Cli,
    label: &str,
    spec: Option<&Value>,
    expected: &[Value],
    query: &str,
    extensions: &str,
    flags: &[&str],
) -> Vec<Value> {
    let run = cli.run(&cli.root.join("search"), query, extensions, spec, flags);
    run.accepted("completed");
    assert!(run.issues.is_empty(), "{label}: {:?}", run.issues);
    let mut wanted: Vec<_> = expected.iter().map(canonical).collect();
    wanted.sort();
    assert_eq!(places(&run.hits), wanted, "{label}");
    println!("PASS {label}");
    run.hits
}
#[test]
fn condition_cases() {
    let cli = Cli::new("conditions");
    docs_search_test_support::specialized::conditions(&cli.root).unwrap();
    let and = condition("excelRow", &["顧客", "必須"], &[], &[]);
    let rows = vec![row(12, "Items"), row(15, "Items")];
    let hits = accepted(&cli, "row AND", Some(&and), &rows, "", "xlsx", &[]);
    for (hit, n) in hits.iter().zip([12, 15]) {
        let evidence: Value = Value::Object(
            hit["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    (
                        v["term"].as_str().unwrap().into(),
                        v["location"]["cellAddress"].clone(),
                    )
                })
                .collect(),
        );
        assert_eq!(
            evidence,
            json!({"顧客":format!("A{n}"),"必須":format!("D{n}")})
        );
    }
    accepted(
        &cli,
        "unit boundary",
        Some(&condition("unit", &["顧客", "必須"], &[], &[])),
        &[],
        "",
        "xlsx",
        &[],
    );
    let file = condition("file", &["顧客", "必須"], &[], &[]);
    let files = vec![
        place(
            "conditions.xlsx",
            "fileMatch",
            json!(null),
            json!(null),
            json!(null),
            json!(null),
        ),
        place(
            "notes.txt",
            "fileMatch",
            json!(null),
            json!(null),
            json!(null),
            json!(null),
        ),
    ];
    accepted(&cli, "file AND", Some(&file), &files, "", "xlsx,txt", &[]);
    let excluded = condition("excelRow", &["顧客", "必須"], &[], &["廃止"]);
    accepted(
        &cli,
        "row NOT",
        Some(&excluded),
        &rows[..1],
        "",
        "xlsx",
        &[],
    );
    let mut or_rows: Vec<_> = (12..=15).map(|n| row(n, "Items")).collect();
    or_rows.push(row(12, "Other"));
    accepted(
        &cli,
        "row OR",
        Some(&condition("excelRow", &[], &["顧客", "必須"], &[])),
        &or_rows,
        "",
        "xlsx",
        &[],
    );
    let name_body = condition("file", &["filename-signal", "body-signal"], &[], &[]);
    accepted(
        &cli,
        "filename and body",
        Some(&name_body),
        &[place(
            "filename-signal.xlsx",
            "fileMatch",
            json!(null),
            json!(null),
            json!(null),
            json!(null),
        )],
        "",
        "xlsx",
        &[],
    );
    for scope in ["unit", "excelRow"] {
        accepted(
            &cli,
            &format!("filename boundary {scope}"),
            Some(&condition(
                scope,
                &["filename-signal", "body-signal"],
                &[],
                &[],
            )),
            &[],
            "",
            "xlsx",
            &[],
        );
    }
    let literal = vec![
        place(
            "conditions.xlsx",
            "cell",
            json!("Items"),
            json!(null),
            json!("B16"),
            json!(null),
        ),
        place(
            "notes.txt",
            "textLine",
            json!(null),
            json!(null),
            json!(null),
            json!(3),
        ),
    ];
    accepted(
        &cli,
        "literal advanced",
        Some(&condition("unit", &["AND OR"], &[], &[])),
        &literal,
        "",
        "xlsx,txt",
        &[],
    );
    accepted(
        &cli,
        "literal normal",
        None,
        &literal,
        "AND OR",
        "xlsx,txt",
        &[],
    );
    let typo = vec![place(
        "conditions.xlsx",
        "cell",
        json!("Items"),
        json!(null),
        json!("A17"),
        json!(null),
    )];
    accepted(
        &cli,
        "typo off",
        Some(&condition("unit", &["custmer"], &[], &[])),
        &[],
        "",
        "xlsx",
        &[],
    );
    let fuzzy = accepted(
        &cli,
        "typo on",
        Some(&condition("unit", &["custmer"], &[], &[])),
        &typo,
        "",
        "xlsx",
        &["--fuzzy-search"],
    );
    assert_eq!(fuzzy[0]["matchCategory"], "fuzzy");
    accepted(
        &cli,
        "NOT stays literal",
        Some(&condition("unit", &["customer"], &[], &["custmer"])),
        &typo,
        "",
        "xlsx",
        &["--fuzzy-search"],
    );
    for (spec, wanted, label, extensions) in [
        (&and, &rows[..], "row AND", "xlsx"),
        (&excluded, &rows[..1], "row NOT", "xlsx"),
        (&file, &files[..], "file AND", "xlsx,txt"),
    ] {
        for indexed in [false, true] {
            for fuzzy in [false, true] {
                let mut flags = vec![];
                if indexed {
                    flags.push("--use-index");
                }
                if fuzzy {
                    flags.push("--fuzzy-search");
                }
                accepted(
                    &cli,
                    &format!("{label} index={indexed} fuzzy={fuzzy}"),
                    Some(spec),
                    wanted,
                    "",
                    extensions,
                    &flags,
                );
            }
        }
    }
    let long = "A".repeat(201);
    for (label, spec, query) in [
        (
            "positive terms required",
            condition("unit", &[], &[], &["A"]),
            "",
        ),
        (
            "all and NOT conflict",
            condition("unit", &["A"], &[], &["a"]),
            "",
        ),
        ("unknown scope", condition("unknown", &["A"], &[], &[]), ""),
        ("term too long", condition("unit", &[&long], &[], &[]), ""),
        (
            "normal and advanced mixed",
            condition("unit", &["A"], &[], &[]),
            "A",
        ),
    ] {
        cli.run(&cli.root.join("search"), query, "xlsx", Some(&spec), &[])
            .rejected("querySpec");
        println!("PASS {label}");
    }
    let run = cli.run(&cli.root.join("errors"), "", "xlsx", Some(&name_body), &[]);
    run.accepted("completed");
    assert!(run.hits.is_empty());
    assert_eq!(run.issues.len(), 1);
    assert_eq!(run.issues[0]["stage"], "read");
    println!("PASS unreadable file scope");
}
