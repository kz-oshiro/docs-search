use docs_search_core::{default_extensions, run_search, SearchRequest};
use std::sync::atomic::AtomicBool;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: docs-search-cli <directory> <query> [--add-directory path]... [--exclude-directory path]... [--extensions xlsx,txt,...] [--cancel-on-start]");
        std::process::exit(2);
    }
    let mut extensions = default_extensions();
    let mut additional_directories = Vec::new();
    let mut excluded_directories = Vec::new();
    let mut cancel_on_start = false;
    let mut options = args[3..].iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--add-directory" | "--exclude-directory" => {
                let Some(path) = options.next() else {
                    eprintln!("{option} requires a directory path");
                    std::process::exit(2);
                };
                if option.as_str() == "--add-directory" {
                    additional_directories.push(path.clone());
                } else {
                    excluded_directories.push(path.clone());
                }
            }
            "--extensions" => {
                let Some(list) = options.next() else {
                    eprintln!("--extensions requires a comma-separated list");
                    std::process::exit(2);
                };
                extensions = if list.is_empty() {
                    Vec::new()
                } else {
                    list.split(',').map(str::to_owned).collect()
                };
            }
            "--cancel-on-start" => cancel_on_start = true,
            _ => {
                eprintln!("Unknown option: {option}");
                std::process::exit(2);
            }
        }
    }
    let request = SearchRequest {
        root_directory: args[1].clone(),
        additional_directories,
        excluded_directories,
        query: args[2].clone(),
        recursive: true,
        extensions,
    };
    let cancel = AtomicBool::new(false);
    if let Err(error) = run_search(request, "cli".into(), &cancel, |event| {
        if cancel_on_start && event.sequence == 1 {
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
