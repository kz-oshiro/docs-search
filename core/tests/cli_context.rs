mod support;
use serde_json::json;
use support::*;
#[test]
fn context_direct_initial_reused() {
    let cli = Cli::new("context");
    docs_search_test_support::specialized::context(&cli.root.join("row-window.xlsx")).unwrap();
    for (query, kind, address, row, column, anchor) in [
        (
            "context-needle",
            "cell",
            json!("B12"),
            json!(12),
            json!(2),
            json!(null),
        ),
        (
            "edge-top",
            "cell",
            json!("A1"),
            json!(1),
            json!(1),
            json!(null),
        ),
        (
            "edge-bottom",
            "cell",
            json!("XFD1048576"),
            json!(1048576),
            json!(16384),
            json!(null),
        ),
        (
            "context-shape",
            "shape",
            json!(null),
            json!(null),
            json!(null),
            json!("B12"),
        ),
        (
            "context-unanchored",
            "shape",
            json!(null),
            json!(null),
            json!(null),
            json!(null),
        ),
    ] {
        let mut prior = None;
        for flags in [vec![], vec!["--use-index"], vec!["--use-index"]] {
            let run = cli.run(&cli.root, query, "xlsx", None, &flags);
            run.accepted("completed");
            assert!(run.issues.is_empty());
            assert_eq!(run.hits.len(), 1);
            let mut hit = run.hits[0].clone();
            assert_eq!(hit["location"]["sheetName"], "Context");
            assert_eq!(
                json!([
                    hit["sourceKind"],
                    hit["location"]["cellAddress"],
                    hit["row"],
                    hit["column"],
                    hit["anchor"]
                ]),
                json!([kind, address, row, column, anchor])
            );
            hit.as_object_mut().unwrap().remove("resultId");
            if let Some(previous) = &prior {
                assert_eq!(&hit, previous);
            }
            prior = Some(hit);
        }
        println!("PASS {query} direct/index initial/index reused");
    }
}
