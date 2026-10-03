//! One-time migration audit; ZIP serialization may differ while entry contracts agree.
use crate::*;
use serde_json::json;
use std::io::Read;
pub fn fixtures(old: &Path, new: &Path) -> Result<Value> {
    let left = files(old)?;
    let right = files(new)?;
    let mut differences = vec![];
    let mut zip_serialization = vec![];
    if left != right {
        differences.push(json!({"kind":"inventory","old":left,"new":right}));
    }
    for name in left.iter().filter(|name| right.contains(name)) {
        let a = fs::read(old.join(name))?;
        let b = fs::read(new.join(name))?;
        let old_zip = zip::ZipArchive::new(std::io::Cursor::new(&a));
        let new_zip = zip::ZipArchive::new(std::io::Cursor::new(&b));
        match (old_zip, new_zip) {
            (Ok(mut old_archive), Ok(mut new_archive)) => {
                if old_archive.len() != new_archive.len() {
                    differences.push(json!({"file":name,"kind":"zip-entry-count"}));
                    continue;
                }
                for i in 0..old_archive.len() {
                    let mut old_entry = old_archive.by_index(i)?;
                    let mut new_entry = new_archive.by_index(i)?;
                    let mut old_bytes = vec![];
                    let mut new_bytes = vec![];
                    old_entry.read_to_end(&mut old_bytes)?;
                    new_entry.read_to_end(&mut new_bytes)?;
                    if old_entry.name() != new_entry.name()
                        || old_entry.compression() != new_entry.compression()
                        || old_entry.last_modified() != new_entry.last_modified()
                        || old_bytes != new_bytes
                    {
                        differences.push(json!({"file":name,"kind":"zip-entry","index":i,"oldEntry":old_entry.name(),"newEntry":new_entry.name(),"oldCompression":format!("{:?}",old_entry.compression()),"newCompression":format!("{:?}",new_entry.compression()),"oldTimestamp":format!("{:?}",old_entry.last_modified()),"newTimestamp":format!("{:?}",new_entry.last_modified()),"sameContent":old_bytes==new_bytes}));
                    }
                }
                if a != b {
                    zip_serialization.push(json!({"file":name,"oldSha256":digest(&old.join(name))?,"newSha256":digest(&new.join(name))?,"oldBytes":a.len(),"newBytes":b.len(),"note":"compressed stream or ZIP header metadata differ; inspect separately from entry contract"}));
                }
            }
            (Err(_), Err(_)) => {
                if a != b {
                    differences.push(json!({"file":name,"kind":"non-zip-bytes","oldSha256":digest(&old.join(name))?,"newSha256":digest(&new.join(name))?}));
                }
            }
            _ => differences.push(json!({"file":name,"kind":"zip-validity"})),
        }
    }
    Ok(
        json!({"schemaVersion":1,"sameContract":differences.is_empty(),"oldRoot":old,"newRoot":new,"differences":differences,"zipSerializationDifferences":zip_serialization}),
    )
}
