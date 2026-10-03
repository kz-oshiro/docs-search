//! Edits address the full source line, never the clipped preview.
use crate::{SearchHit, Unit, OFFICE_EXTENSIONS, SUPPORTED_EXTENSIONS};
use encoding_rs::SHIFT_JIS;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_BYTES: u64 = 32 * 1024 * 1024;
static NEXT_FILE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditAnchor {
    pub line_number: usize,
    pub range: [usize; 2],
    pub source_revision: String,
    pub line_revision: String,
}

pub fn revision(bytes: &[u8]) -> String {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hash);
    format!("{}:{:016x}", bytes.len(), hash.finish())
}

pub(crate) fn anchor(
    unit: &Unit,
    range: [usize; 2],
    source_revision: Option<&str>,
) -> Option<EditAnchor> {
    if unit.source_kind != "textLine" {
        return None;
    }
    Some(EditAnchor {
        line_number: usize::try_from(unit.location["lineNumber"].as_u64()?).ok()?,
        range,
        source_revision: source_revision?.into(),
        line_revision: revision(unit.text.as_bytes()),
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditView {
    pub file_path: String,
    pub line_number: usize,
    pub selected_text: String,
    pub before: String,
    pub after: String,
    pub encoding: &'static str,
}

pub struct EditDraft {
    path: PathBuf,
    bytes: Vec<u8>,
    start: usize,
    end: usize,
    shift_jis: bool,
    newline: &'static str,
}

fn read_limited(file: &mut File) -> Result<Vec<u8>, String> {
    if file
        .metadata()
        .map_err(|_| "ファイル情報を取得できません。")?
        .len()
        > MAX_BYTES
    {
        return Err("編集対象が32MiBを超えています。".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "元ファイルを読み取れません。")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("編集対象が32MiBを超えています。".into());
    }
    Ok(bytes)
}

fn encode(text: &str, shift_jis: bool) -> Result<Vec<u8>, String> {
    if !shift_jis {
        return Ok(text.as_bytes().to_vec());
    }
    let (bytes, _, errors) = SHIFT_JIS.encode(text);
    if errors {
        return Err("Shift_JISで保存できない文字があります。入力を修正してください。".into());
    }
    Ok(bytes.into_owned())
}

pub fn prepare(
    hit: &SearchHit,
    evidence_index: Option<usize>,
) -> Result<(EditDraft, EditView), String> {
    let anchor = if hit.evidence.is_empty() {
        if evidence_index.is_some() {
            return Err("編集する根拠が不正です。".into());
        }
        hit.edit_anchor.as_ref()
    } else {
        hit.evidence
            .get(evidence_index.ok_or("編集する根拠を選んでください。")?)
            .ok_or("編集する根拠が不正です。")?
            .edit_anchor
            .as_ref()
    }
    .ok_or(
        "この形式または検索箇所は直接編集に対応していません。元ファイルを開いて編集してください。",
    )?;
    let path = PathBuf::from(&hit.file_path);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if OFFICE_EXTENSIONS.contains(&extension.as_str())
        || !SUPPORTED_EXTENSIONS.contains(&extension.as_str())
    {
        return Err("この形式は直接編集に対応していません。".into());
    }
    if fs::symlink_metadata(&path)
        .map_err(|_| "元ファイルが見つかりません。再検索してください。")?
        .file_type()
        .is_symlink()
    {
        return Err("リンク先のファイルは直接編集できません。".into());
    }
    let mut file = File::open(&path).map_err(|_| "元ファイルを開けません。再検索してください。")?;
    if file
        .metadata()
        .map_err(|_| "ファイル情報を取得できません。")?
        .permissions()
        .readonly()
    {
        return Err(
            "読み取り専用ファイルです。書き込み可能な状態にして再検索してください。".into(),
        );
    }
    let bytes = read_limited(&mut file)?;
    if revision(&bytes) != anchor.source_revision {
        return Err("検索後にファイルが変更されました。再検索してください。".into());
    }
    let bom = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        3
    } else {
        0
    };
    let (text, shift_jis) = match std::str::from_utf8(&bytes[bom..]) {
        Ok(text) => (text.to_owned(), false),
        Err(_) if bom == 0 => (
            SHIFT_JIS
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .ok_or("元ファイルの文字コードを読み取れません。")?
                .into_owned(),
            true,
        ),
        Err(_) => return Err("UTF-8の内容が不正です。".into()),
    };
    let line = text
        .lines()
        .nth(
            anchor
                .line_number
                .checked_sub(1)
                .ok_or("行番号が不正です。")?,
        )
        .ok_or("一致した行が見つかりません。再検索してください。")?;
    if revision(line.as_bytes()) != anchor.line_revision {
        return Err("一致した行の内容が変わりました。再検索してください。".into());
    }
    let chars: Vec<char> = line.chars().collect();
    let [first, last] = anchor.range;
    if first >= last || last > chars.len() {
        return Err("編集範囲を特定できません。再検索してください。".into());
    }
    let line_start = line.as_ptr() as usize - text.as_ptr() as usize;
    let prefix: String = chars[..first].iter().collect();
    let selected: String = chars[first..last].iter().collect();
    let start =
        bom + encode(&text[..line_start], shift_jis)?.len() + encode(&prefix, shift_jis)?.len();
    let end = start + encode(&selected, shift_jis)?.len();
    let original_range = bytes
        .get(start..end)
        .ok_or("元ファイルの編集範囲を特定できません。")?;
    let decoded_range = if shift_jis {
        SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(original_range)
            .map(|text| text.into_owned())
    } else {
        std::str::from_utf8(original_range).ok().map(str::to_owned)
    };
    if decoded_range.as_deref() != Some(selected.as_str()) {
        return Err(
            "元ファイルの文字と編集範囲を対応づけられません。元ファイルを開いて編集してください。"
                .into(),
        );
    }
    let view = EditView {
        file_path: hit.file_path.clone(),
        line_number: anchor.line_number,
        selected_text: selected,
        before: chars[first.saturating_sub(150)..first].iter().collect(),
        after: chars[last..(last + 150).min(chars.len())].iter().collect(),
        encoding: if shift_jis {
            "Shift_JIS"
        } else if bom > 0 {
            "UTF-8 BOM"
        } else {
            "UTF-8"
        },
    };
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    Ok((
        EditDraft {
            path,
            bytes,
            start,
            end,
            shift_jis,
            newline,
        },
        view,
    ))
}

impl EditDraft {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn save(&self, replacement: &str) -> Result<(), String> {
        if fs::symlink_metadata(&self.path)
            .map_err(|_| "元ファイルが見つかりません。再検索してください。")?
            .file_type()
            .is_symlink()
        {
            return Err("元ファイルがリンクへ変更されました。再検索してください。".into());
        }
        // Hold a Windows handle that excludes writers while checking and replacing.
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(1 | 4); // FILE_SHARE_READ | FILE_SHARE_DELETE
        }
        let mut source = options
            .open(&self.path)
            .map_err(|_| "元ファイルを開けません。他のアプリで開いている場合は閉じてください。")?;
        let permissions = source
            .metadata()
            .map_err(|_| "ファイル情報を取得できません。")?
            .permissions();
        if permissions.readonly() {
            return Err("読み取り専用のため保存できません。".into());
        }
        if read_limited(&mut source)? != self.bytes {
            return Err(
                "編集中に元ファイルが変更されました。入力を控えてから再検索してください。".into(),
            );
        }
        let normalized = replacement
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\n', self.newline);
        let encoded = encode(&normalized, self.shift_jis)?;
        if self
            .bytes
            .len()
            .saturating_sub(self.end - self.start)
            .saturating_add(encoded.len()) as u64
            > MAX_BYTES
        {
            return Err("保存後のファイルが32MiBを超えます。".into());
        }
        let temporary = self.path.with_file_name(format!(
            ".docs-search-edit-{}-{}.tmp",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut created = false;
        let result = (|| {
            let mut file = OpenOptions::new().create_new(true).write(true).open(&temporary)
                .map_err(|_| "同じフォルダーに保存用ファイルを作成できません。書き込み権限を確認してください。")?;
            created = true;
            file.write_all(&self.bytes[..self.start])
                .and_then(|_| file.write_all(&encoded))
                .and_then(|_| file.write_all(&self.bytes[self.end..]))
                .and_then(|_| file.sync_all())
                .map_err(|_| "保存用ファイルへ書き込めません。空き容量を確認してください。")?;
            drop(file);
            fs::set_permissions(&temporary, permissions)
                .map_err(|_| "元ファイルの属性を保持できません。")?;
            // Other editors may replace the path instead of writing the open file.
            let mut current = options.open(&self.path).map_err(|_| {
                "保存直前に元ファイルを確認できません。入力を控えて再検索してください。"
            })?;
            if read_limited(&mut current)? != self.bytes {
                return Err(
                    "保存直前に元ファイルが変更されました。入力を控えて再検索してください。".into(),
                );
            }
            replace_file(&temporary, &self.path)
        })();
        if created && result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

#[cfg(windows)]
fn replace_file(temporary: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn ReplaceFileW(
            replaced: *const u16,
            replacement: *const u16,
            backup: *const u16,
            flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
    }
    let backup = temporary.with_extension("bak");
    if backup.exists() {
        return Err("保存用バックアップ名が重複しました。再度保存してください。".into());
    }
    let target_full =
        fs::canonicalize(target).map_err(|_| "置換対象が見つかりません。再検索してください。")?;
    let temporary_full =
        fs::canonicalize(temporary).map_err(|_| "保存用ファイルが見つかりません。")?;
    let backup_full = temporary_full.with_extension("bak");
    let target_wide: Vec<u16> = target_full
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let backup_wide: Vec<u16> = backup_full
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let temporary: Vec<u16> = temporary_full
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let success = unsafe {
        ReplaceFileW(
            target_wide.as_ptr(),
            temporary.as_ptr(),
            backup_wide.as_ptr(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if success != 0 {
        let _ = fs::remove_file(&backup);
        return Ok(());
    }
    if backup.exists() {
        if !target.exists() && fs::rename(&backup, target).is_ok() {
            return Err("保存に失敗したため元ファイルを復元しました。書き込み権限と他アプリの使用状態を確認してください。".into());
        }
        return Err(format!("保存に失敗しました。元の内容のバックアップを保持しています: {}。入力内容も保持しています。",backup.display()));
    }
    Err(
        "元ファイルを置き換えられません。書き込み権限と他アプリの使用状態を確認してください。"
            .into(),
    )
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, target: &Path) -> Result<(), String> {
    fs::rename(temporary, target).map_err(|_| "元ファイルを置き換えられません。".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run_search, EventKind, SearchRequest};
    use std::sync::atomic::AtomicBool;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new(bytes: &[u8]) -> Self {
            let path = std::env::temp_dir().join(format!(
                "docs-search-edit-{}-{}",
                std::process::id(),
                NEXT_FILE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::write(path.join("sample.txt"), bytes).unwrap();
            Self(path)
        }
        fn hit(&self, query: &str, spec: Option<serde_json::Value>) -> SearchHit {
            let mut hits = Vec::new();
            run_search(
                SearchRequest {
                    root_directory: self.0.to_string_lossy().into_owned(),
                    additional_directories: vec![],
                    excluded_directories: vec![],
                    query: query.into(),
                    query_spec: spec,
                    recursive: true,
                    extensions: vec!["txt".into()],
                    use_index: false,
                    fuzzy_search: false,
                    include_notes: false,
                    include_formulas: false,
                },
                "edit-test".into(),
                &AtomicBool::new(false),
                |event| {
                    if let EventKind::Result { hit } = event.kind {
                        if hit.source_kind != "fileName" {
                            hits.push(hit);
                        }
                    }
                },
            )
            .unwrap();
            hits.remove(0)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn replaces_selected_unicode_range_preserving_bom_crlf_and_other_occurrences() {
        let original = "\u{feff}前文\r\n顧客 顧客 😀\r\n後文\r\n";
        let fixture = Fixture::new(original.as_bytes());
        let hit = fixture.hit("顧客", None);
        let (draft, view) = prepare(&hit, None).unwrap();
        assert_eq!(view.selected_text, "顧客");
        draft.save("注文").unwrap();
        assert_eq!(
            fs::read(fixture.0.join("sample.txt")).unwrap(),
            "\u{feff}前文\r\n注文 顧客 😀\r\n後文\r\n".as_bytes()
        );
    }
    #[test]
    fn full_source_ranges_work_beyond_clipped_preview_and_cancelling_changes_nothing() {
        let original = format!("{}needle suffix\n", "あ".repeat(400));
        let fixture = Fixture::new(original.as_bytes());
        let hit = fixture.hit("needle", None);
        assert!(hit.preview_truncated);
        let (draft, view) = prepare(&hit, None).unwrap();
        assert_eq!(view.selected_text, "needle");
        drop(draft);
        assert_eq!(
            fs::read(fixture.0.join("sample.txt")).unwrap(),
            original.as_bytes()
        );
        let (draft, _) = prepare(&hit, None).unwrap();
        draft.save("changed").unwrap();
        assert_eq!(
            fs::read_to_string(fixture.0.join("sample.txt")).unwrap(),
            original.replacen("needle", "changed", 1)
        );
    }
    #[test]
    fn rejects_changes_before_prepare_and_save_without_overwriting() {
        let fixture = Fixture::new(b"needle original\n");
        let hit = fixture.hit("needle", None);
        let (draft, _) = prepare(&hit, None).unwrap();
        let external = b"needle EXTERNAL\n";
        fs::write(fixture.0.join("sample.txt"), external).unwrap();
        assert!(prepare(&hit, None).is_err());
        assert!(draft.save("replacement").is_err());
        assert_eq!(fs::read(fixture.0.join("sample.txt")).unwrap(), external);
    }
    #[test]
    fn preserves_shift_jis_and_rejects_unrepresentable_replacement() {
        let original = SHIFT_JIS.encode("顧客 顧客\r\nそのまま\r\n").0.into_owned();
        let fixture = Fixture::new(&original);
        let hit = fixture.hit("顧客", None);
        let (draft, view) = prepare(&hit, None).unwrap();
        assert_eq!(view.encoding, "Shift_JIS");
        assert!(draft.save("😀").is_err());
        assert_eq!(fs::read(fixture.0.join("sample.txt")).unwrap(), original);
        draft.save("注文").unwrap();
        assert_eq!(
            fs::read(fixture.0.join("sample.txt")).unwrap(),
            SHIFT_JIS.encode("注文 顧客\r\nそのまま\r\n").0.as_ref()
        );
    }
    #[test]
    fn advanced_evidence_selects_only_chosen_term() {
        let fixture = Fixture::new(b"alpha beta\n");
        let hit=fixture.hit("",Some(serde_json::json!({"mode":"conditions","scope":"unit","all":["alpha","beta"],"any":[],"not":[]})));
        let index = hit
            .evidence
            .iter()
            .position(|item| item.term == "beta")
            .unwrap();
        assert!(prepare(&hit, None).is_err());
        let (draft, view) = prepare(&hit, Some(index)).unwrap();
        assert_eq!(view.selected_text, "beta");
        draft.save("gamma").unwrap();
        assert_eq!(
            fs::read(fixture.0.join("sample.txt")).unwrap(),
            b"alpha gamma\n"
        );
    }
}
