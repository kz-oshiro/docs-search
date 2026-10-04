//! Development-only deterministic corpora. No dependency on the search engine.
pub mod common;
pub mod icons;
mod random;
pub mod specialized;

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const SEED: u64 = 20260927;
pub const CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
pub const P: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
pub const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub const S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const XDR: &str = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";
pub const PPT: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
pub const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub type Entries = Vec<(String, Vec<u8>)>;

pub fn xml(text: impl std::fmt::Display) -> String {
    text.to_string()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub fn template(group: &str, key: &str, args: &[&str]) -> String {
    static TEMPLATES: OnceLock<Value> = OnceLock::new();
    let templates = TEMPLATES.get_or_init(|| serde_json::json!({
        "common": serde_json::from_str::<Value>(include_str!("../templates/common.json")).unwrap(),
        "context": serde_json::from_str::<Value>(include_str!("../templates/context.json")).unwrap(),
        "conditions": serde_json::from_str::<Value>(include_str!("../templates/conditions.json")).unwrap(),
        "office": serde_json::from_str::<Value>(include_str!("../templates/office.json")).unwrap(),
        "issues": serde_json::from_str::<Value>(include_str!("../templates/issues.json")).unwrap()
    }));
    let source = templates[group][key].as_str().expect("source template");
    // A single pass prevents placeholder-looking user text from being expanded again.
    let mut result = String::new();
    let mut rest = source;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let end = rest[start..].find('}').expect("template delimiter") + start;
        let index: usize = rest[start + 2..end].parse().expect("template index");
        result.push_str(args[index]);
        rest = &rest[end + 1..];
    }
    result.push_str(rest);
    result
}
pub fn put(entries: &mut Entries, name: &str, bytes: impl AsRef<[u8]>) {
    if let Some(entry) = entries.iter_mut().find(|entry| entry.0 == name) {
        entry.1 = bytes.as_ref().to_vec();
    } else {
        entries.push((name.into(), bytes.as_ref().to_vec()));
    }
}
pub fn write(path: &Path, bytes: impl AsRef<[u8]>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}
pub fn write_text(path: &Path, text: &str) -> Result<()> {
    // Preserve the old Python write_text platform newline contract.
    write(
        path,
        if cfg!(windows) {
            text.replace('\n', "\r\n")
        } else {
            text.to_owned()
        },
    )
}
pub fn write_json(path: &Path, value: &Value) -> Result<()> {
    write_text(path, &(serde_json::to_string_pretty(value)? + "\n"))
}
pub fn package(path: &Path, entries: Entries, deflate: bool, unix: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut zip = zip::ZipWriter::new(fs::File::create(path)?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(if deflate {
            zip::CompressionMethod::Deflated
        } else {
            zip::CompressionMethod::Stored
        })
        .last_modified_time(zip::DateTime::from_date_and_time(2020, 1, 1, 0, 0, 0).unwrap())
        .unix_permissions(0o600);
    for (name, bytes) in entries {
        zip.start_file(name, options)?;
        zip.write_all(&bytes)?;
    }
    zip.finish()?;
    // zip's writer uses Unix metadata. Preserve the Python corpus's MS-DOS creator
    // for common/Office fixtures; issue fixtures retain Unix creator metadata.
    {
        let mut bytes = fs::read(path)?;
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes))?;
        let offsets: Vec<_> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().central_header_start() as usize)
            .collect();
        drop(archive);
        for offset in offsets {
            bytes[offset + 5] = if unix { 3 } else { 0 };
            bytes[offset + 38..offset + 42].copy_from_slice(&0x01800000u32.to_le_bytes());
        }
        fs::write(path, bytes)?;
    }
    Ok(())
}
pub fn files(root: &Path) -> Result<Vec<String>> {
    fn visit(root: &Path, dir: &Path, items: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                visit(root, &path, items)?;
            } else {
                items.push(
                    path.strip_prefix(root)?
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
        Ok(())
    }
    let mut items = vec![];
    visit(root, root, &mut items)?;
    items.sort();
    Ok(items)
}
pub fn digest(path: &Path) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
pub struct TempDir(PathBuf);
impl TempDir {
    pub fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "docs-search-{label}-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("temporary directory");
        Self(path)
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
