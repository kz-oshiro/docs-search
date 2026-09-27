use doc_search_core::{run_search, SearchRequest};
use std::sync::atomic::AtomicBool;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 && !(args.len() == 4 && args[3] == "--cancel-on-start") {
        eprintln!("Usage: doc-search-cli <directory> <query> [--cancel-on-start]");
        std::process::exit(2);
    }
    let request = SearchRequest {
        root_directory: args[1].clone(),
        query: args[2].clone(),
        recursive: true,
    };
    let cancel = AtomicBool::new(false);
    if let Err(error) = run_search(request, "cli".into(), &cancel, |event| {
        if args.len() == 4 && event.sequence == 1 {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        println!(
            "{}",
            serde_json::to_string(&event).expect("event serialization")
        );
    }) {
        eprintln!("{}: {}", error.field, error.message);
        std::process::exit(2);
    }
}
