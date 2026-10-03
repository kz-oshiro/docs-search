use docs_search_test_support::*;
use serde_json::Value;
use std::{collections::BTreeMap, fs, io::Read, path::Path};
fn hashes(root: &Path) -> BTreeMap<String, String> {
    files(root)
        .unwrap()
        .into_iter()
        .map(|name| {
            let hash = digest(&root.join(&name)).unwrap();
            (name, hash)
        })
        .collect()
}
fn valid_office(path: &Path) {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    assert!(archive.by_name("[Content_Types].xml").is_ok());
    assert!(archive.by_name("_rels/.rels").is_ok());
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let mut bytes = vec![];
        entry.read_to_end(&mut bytes).unwrap(); // also checks CRC
        assert_eq!(
            entry.last_modified().unwrap(),
            zip::DateTime::from_date_and_time(2020, 1, 1, 0, 0, 0).unwrap()
        );
        assert_eq!(entry.compression(), zip::CompressionMethod::Stored);
        if ["xml", "rels", "vml"].contains(&entry.name().rsplit('.').next().unwrap()) {
            roxmltree::Document::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
        }
    }
}
fn office_names(manifest: &Value) -> Vec<&str> {
    manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .filter(|name| {
            ["xlsx", "xlsm", "pptx", "docx"].contains(&name.rsplit('.').next().unwrap())
                && !name.contains("/errors/")
                && !name.ends_with("broken.xlsx")
                && !name.contains("/~$")
        })
        .collect()
}
#[test]
fn deterministic_acceptance_and_inventory() {
    let temp = TempDir::new("generator");
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    let manifest = common::generate(&first, "acceptance").unwrap();
    common::generate(&second, "acceptance").unwrap();
    assert_eq!(hashes(&first), hashes(&second));
    let cases: Value =
        serde_json::from_slice(include_bytes!("../../tests/fixtures/backend-cases.json")).unwrap();
    assert_eq!(manifest["seed"], cases["seed"]);
    let names: Vec<_> = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 33);
    assert!(names.iter().all(|n| n.starts_with("search/")));
    assert_eq!(names.iter().filter(|n| n.ends_with(".xlsx")).count(), 4);
    for suffix in ["jsp", "xhtml", "html", "js", "java"] {
        assert_eq!(
            names
                .iter()
                .filter(|n| n.starts_with("search/code/") && n.ends_with(&format!(".{suffix}")))
                .count(),
            if suffix == "html" { 2 } else { 1 }
        );
    }
    for suffix in [
        "vue", "mjs", "jsx", "tsx", "json", "md", "csv", "py", "yaml", "svg", "ipynb",
    ] {
        assert_eq!(
            names
                .iter()
                .filter(|n| n.starts_with("search/extended/") && n.ends_with(&format!(".{suffix}")))
                .count(),
            1
        );
    }
    for name in ["notes/shift-jis.txt", "code/shift-jis.html"] {
        let bytes = fs::read(first.join("search").join(name)).unwrap();
        let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode("日本語①");
        assert!(!errors);
        assert!(bytes.windows(encoded.len()).any(|w| w == encoded.as_ref()));
        assert!(!bytes
            .windows("日本語①".len())
            .any(|w| w == "日本語①".as_bytes()));
    }
    for case in cases["cases"].as_array().unwrap() {
        let expected = &case["expected"];
        if let Some(counts) = expected.get("counts") {
            assert_eq!(
                counts["resultCount"].as_u64().unwrap() as usize,
                expected["results"].as_array().unwrap().len()
            );
            assert_eq!(
                counts["issueCount"].as_u64().unwrap() as usize,
                expected["issues"].as_array().unwrap().len()
            );
        }
        for field in ["results", "issues"] {
            for item in expected[field].as_array().into_iter().flatten() {
                assert!(
                    first
                        .join("search")
                        .join(item["file"].as_str().unwrap())
                        .is_file(),
                    "{}",
                    case["id"]
                );
            }
        }
    }
    let office = office_names(&manifest);
    assert_eq!(office.len(), 5);
    for name in office {
        valid_office(&first.join(name));
    }
    for (name, min) in [
        ("spreadsheets/needle-book.xlsx", 100000),
        ("nested/team-briefing.pptx", 5000),
        ("documents/operations-guide.docx", 5000),
        ("notes/research-log.txt", 10000),
    ] {
        assert!(fs::metadata(first.join("search").join(name)).unwrap().len() > min);
    }
    assert!(
        common::generate(&first, "acceptance").is_err(),
        "existing output must never be overwritten"
    );
}
#[test]
fn load_inventory_and_sizes() {
    let temp = TempDir::new("load");
    let root = std::env::var_os("DOCS_SEARCH_TEST_CORPUS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temp.path().join("corpus"));
    let manifest = if root.exists() {
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap()
    } else {
        common::generate(&root, "load").unwrap()
    };
    assert_eq!(manifest["profile"], "load");
    let names = manifest["files"].as_array().unwrap();
    assert_eq!(names.len(), 148);
    assert_eq!(
        names
            .iter()
            .filter(|v| v.as_str().unwrap().starts_with("search/"))
            .count(),
        33
    );
    for (suffix, count) in [("xlsx", 32), ("pptx", 32), ("docx", 31), ("txt", 20)] {
        assert_eq!(
            names
                .iter()
                .filter(|v| {
                    let s = v.as_str().unwrap();
                    s.starts_with("load/") && s.ends_with(&format!(".{suffix}"))
                })
                .count(),
            count
        );
    }
    let office = office_names(&manifest);
    assert_eq!(office.len(), 100);
    for name in office {
        let size = fs::metadata(root.join(name)).unwrap().len();
        assert!((450000..=550000).contains(&size), "{name}: {size}");
    }
    assert_eq!(
        fs::read(root.join("backend-cases.json")).unwrap(),
        include_bytes!("../../tests/fixtures/backend-cases.json")
    );
}
