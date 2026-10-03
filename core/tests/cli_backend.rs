mod support;
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path};
use support::*;
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

#[test]
fn extension_catalog() {
    let html = include_str!("../../frontend/index.html");
    let inputs: Vec<_> = html
        .split('<')
        .filter(|s| s.starts_with("input ") && s.contains("name=\"extension\""))
        .map(|s| {
            assert!(s.split('>').next().unwrap().contains("checked"));
            s.split("value=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
        })
        .collect();
    let supported = docs_search_core::SUPPORTED_EXTENSIONS;
    assert_eq!(inputs.len(), inputs.iter().collect::<BTreeSet<_>>().len());
    assert_eq!(
        supported.len(),
        supported.iter().collect::<BTreeSet<_>>().len()
    );
    assert_eq!(
        inputs.into_iter().collect::<BTreeSet<_>>(),
        supported.iter().copied().collect()
    );
}
#[test]
fn common_cases() {
    let mut cli = Cli::new("backend");
    if let Some(root) = std::env::var_os("DOCS_SEARCH_TEST_CORPUS") {
        cli.root = root.into();
    } else {
        docs_search_test_support::common::generate(&cli.root, "load").unwrap();
    }
    let spec: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/backend-cases.json")).unwrap();
    assert_eq!(spec["cases"].as_array().unwrap().len(), 27);
    for case in spec["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let request = case.get("request").unwrap_or(&case["steps"][0]["request"]);
        let replace = |v: &Value| {
            v.as_str()
                .unwrap()
                .replace("${fixtureRoot}", &cli.root.to_string_lossy())
        };
        let folder = replace(&request["rootDirectory"]);
        let mut extra = vec![];
        for (field, flag) in [
            ("additionalDirectories", "--add-directory"),
            ("excludedDirectories", "--exclude-directory"),
        ] {
            for directory in request[field].as_array().into_iter().flatten() {
                extra.extend([flag.into(), replace(directory)]);
            }
        }
        if let Some(extensions) = request["extensions"].as_array() {
            extra.extend([
                "--extensions".into(),
                extensions
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join(","),
            ]);
        }
        let mut flags = vec![];
        for (field, flag) in [
            ("useIndex", "--use-index"),
            ("fuzzySearch", "--fuzzy-search"),
        ] {
            if request[field] == true {
                flags.push(flag);
            }
        }
        if id == "cancel-load-search" {
            flags.push("--cancel-on-start");
        }
        let run = cli.run_args(
            Path::new(&folder),
            request["query"].as_str().unwrap(),
            &extra,
            None,
            &flags,
        );
        let expected = &case["expected"];
        if expected["accepted"] != true {
            run.rejected(expected["inputErrorField"].as_str().unwrap());
            println!("PASS {id}");
            continue;
        }
        run.accepted(expected["finished"].as_str().unwrap());
        let actual_request = &run.events[0]["request"];
        assert_eq!(
            actual_request["extensions"],
            request
                .get("extensions")
                .cloned()
                .unwrap_or(json!(["xlsx", "xlsm", "pptx", "docx", "txt"])),
            "{id}"
        );
        for field in ["useIndex", "fuzzySearch"] {
            assert_eq!(
                actual_request[field],
                request.get(field).cloned().unwrap_or(json!(false)),
                "{id}"
            );
        }
        for field in ["additionalDirectories", "excludedDirectories"] {
            assert_eq!(
                actual_request[field],
                json!(request[field]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(replace)
                    .collect::<Vec<_>>()),
                "{id}"
            );
        }
        let needle = request["query"]
            .as_str()
            .unwrap()
            .trim()
            .nfc()
            .collect::<String>()
            .case_fold()
            .collect::<String>();
        for hit in &run.hits {
            let characters: Vec<_> = hit["previewText"].as_str().unwrap().chars().collect();
            let ranges = hit["matchRanges"].as_array().unwrap();
            assert!(!ranges.is_empty());
            let mut previous = 0;
            for range in ranges {
                let start = range[0].as_u64().unwrap() as usize;
                let end = range[1].as_u64().unwrap() as usize;
                assert!(
                    previous <= start && start < end && end <= characters.len(),
                    "{id}"
                );
                if ["exact", "caseFolded"].contains(&hit["matchType"].as_str().unwrap()) {
                    assert!(
                        characters[start..end]
                            .iter()
                            .collect::<String>()
                            .nfc()
                            .collect::<String>()
                            .case_fold()
                            .collect::<String>()
                            .contains(&needle),
                        "{id}"
                    );
                }
                previous = end;
            }
        }
        if id == "cancel-load-search" {
            println!("PASS {id}");
            continue;
        }
        assert_eq!(
            run.events.last().unwrap()["counts"],
            expected["counts"],
            "{id}"
        );
        assert_eq!(
            run.hits.len(),
            expected["results"].as_array().unwrap().len(),
            "{id}"
        );
        let relative = |v: &Value| {
            Path::new(v.as_str().unwrap())
                .strip_prefix(cli.root.join("search"))
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        };
        let key = |h: &Value| {
            let mut location = h["location"].clone();
            location.as_object_mut().unwrap().remove("sheetIndex");
            canonical(&json!([
                relative(&h["filePath"]),
                h["sourceKind"],
                location
            ]))
        };
        assert_eq!(
            run.hits.iter().map(key).collect::<BTreeSet<_>>().len(),
            run.hits.len(),
            "{id}"
        );
        for wanted in expected["results"].as_array().unwrap() {
            let wanted_key = canonical(&json!([
                wanted["file"],
                wanted["sourceKind"],
                wanted["location"]
            ]));
            let hit = run
                .hits
                .iter()
                .find(|h| key(h) == wanted_key)
                .unwrap_or_else(|| panic!("{id}: missing {wanted}"));
            assert!(
                hit["previewText"]
                    .as_str()
                    .unwrap()
                    .contains(wanted["textContains"].as_str().unwrap()),
                "{id}"
            );
            for field in ["matchType", "matchCategory", "score"] {
                if let Some(value) = wanted.get(field) {
                    assert_eq!(&hit[field], value, "{id}");
                }
            }
        }
        let actual: BTreeSet<_> = run
            .issues
            .iter()
            .map(|v| canonical(&json!([relative(&v["path"]), v["stage"], v["code"]])))
            .collect();
        let wanted: BTreeSet<_> = expected["issues"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| canonical(&json!([v["file"], v["stage"], v["code"]])))
            .collect();
        assert_eq!(run.issues.len(), wanted.len(), "{id}");
        assert_eq!(actual, wanted, "{id}");
        println!("PASS {id}");
    }
}
