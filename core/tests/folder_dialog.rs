// Compile the desktop's pure decision logic without opening an OS dialog.
#[path = "../../src-tauri/src/folder_dialog.rs"]
mod folder_dialog;

use docs_search_test_support::TempDir;
use std::fs;

#[test]
fn existing_directory_itself_wins_over_executable_parent() {
    let temp = TempDir::new("folder-dialog");
    let directory = temp.path().join("日本語 空白");
    fs::create_dir(&directory).unwrap();
    let executable = temp.path().join("app.exe");
    for input in [
        directory.to_string_lossy().into_owned(),
        format!("  {}  ", directory.display()),
    ] {
        assert_eq!(
            folder_dialog::initial_directory(Some(&input), Some(&executable)),
            Some(directory.clone())
        );
        assert_eq!(
            folder_dialog::initial_directory(Some(&input), None),
            Some(directory.clone())
        );
    }
}

#[test]
fn unusable_inputs_use_executable_parent_or_dialog_default() {
    let temp = TempDir::new("folder-dialog-fallback");
    let file = temp.path().join("文書.txt");
    fs::write(&file, "unchanged").unwrap();
    let missing = temp.path().join("missing");
    let executable = temp.path().join("app.exe");
    for input in [
        None,
        Some(""),
        Some(" \t "),
        missing.to_str(),
        file.to_str(),
    ] {
        assert_eq!(
            folder_dialog::initial_directory(input, Some(&executable)),
            Some(temp.path().to_path_buf())
        );
        assert_eq!(folder_dialog::initial_directory(input, None), None);
        assert_eq!(
            folder_dialog::initial_directory(input, Some(std::path::Path::new("/"))),
            None
        );
    }
    assert_eq!(fs::read_to_string(file).unwrap(), "unchanged");
}
