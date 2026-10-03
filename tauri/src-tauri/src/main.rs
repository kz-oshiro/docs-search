#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use docs_search_core::context::{self, ContextError, ContextRange, ContextTarget, ResultContext};
use docs_search_core::edit::{self, EditDraft, EditView};
use docs_search_core::report::{self, Format, Report};
use docs_search_core::{run_search, validate, EventKind, InputError, SearchHit, SearchRequest};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

struct ActiveSearch {
    id: String,
    cancel: Arc<AtomicBool>,
}
struct SearchSession {
    id: String,
    targets: HashMap<usize, ContextTarget>,
    hits: HashMap<usize, SearchHit>,
    drafts: HashMap<String, EditDraft>,
    edited_paths: HashSet<String>,
}
#[derive(Default)]
struct SearchSessions {
    active: Option<ActiveSearch>,
    latest: Option<SearchSession>,
}
struct SearchState(Mutex<SearchSessions>);
static NEXT_EDIT: AtomicU64 = AtomicU64::new(1);

fn context_error(code: &'static str, message: &str) -> ContextError {
    ContextError {
        code,
        message: message.into(),
    }
}

#[tauri::command]
async fn pick_folder() -> Option<String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut dialog = rfd::FileDialog::new();
        if let Ok(executable) = std::env::current_exe() {
            if let Some(directory) = executable.parent() {
                dialog = dialog.set_directory(directory);
            }
        }
        dialog
            .pick_folder()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
fn start_search(
    request: SearchRequest,
    search_id: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, SearchState>,
) -> Result<String, InputError> {
    validate(&request)?;
    let mut current = state.0.lock().map_err(|_| InputError {
        field: "general",
        message: "検索状態を取得できません。",
    })?;
    if current.active.is_some() {
        return Err(InputError {
            field: "general",
            message: "前の検索が終了するまでお待ちください。",
        });
    }
    if search_id.is_empty() || search_id.len() > 100 {
        return Err(InputError {
            field: "general",
            message: "検索 ID が不正です。",
        });
    }
    let cancel = Arc::new(AtomicBool::new(false));
    current.latest = Some(SearchSession {
        id: search_id.clone(),
        targets: HashMap::new(),
        hits: HashMap::new(),
        drafts: HashMap::new(),
        edited_paths: HashSet::new(),
    });
    current.active = Some(ActiveSearch {
        id: search_id.clone(),
        cancel: cancel.clone(),
    });
    let id = search_id.clone();
    let use_index = request.use_index;
    std::thread::spawn(move || {
        let app_for_events = app.clone();
        let mut batch = Vec::with_capacity(128);
        let _ = run_search(request, id.clone(), &cancel, |event| {
            if let EventKind::Result { hit } = &event.kind {
                if let Some(state) = app_for_events.try_state::<SearchState>() {
                    if let Ok(mut current) = state.0.lock() {
                        if let Some(session) = current.latest.as_mut().filter(|s| s.id == id) {
                            session.hits.insert(hit.result_id, hit.clone());
                        }
                    }
                }
                if let Some(target) = ContextTarget::from_hit(hit, use_index) {
                    if let Some(state) = app_for_events.try_state::<SearchState>() {
                        if let Ok(mut current) = state.0.lock() {
                            if let Some(session) = current.latest.as_mut().filter(|s| s.id == id) {
                                session.targets.insert(hit.result_id, target);
                            }
                        }
                    }
                }
            }
            let flush = !matches!(&event.kind, EventKind::Result { .. });
            batch.push(event);
            if flush || batch.len() >= 128 {
                let _ = app_for_events.emit("search-events", &batch);
                batch.clear();
            }
        });
        if !batch.is_empty() {
            let _ = app_for_events.emit("search-events", &batch);
        }
        if let Some(state) = app.try_state::<SearchState>() {
            if let Ok(mut current) = state.0.lock() {
                if current.active.as_ref().is_some_and(|s| s.id == id) {
                    current.active = None;
                }
            }
        }
    });
    Ok(search_id)
}

#[tauri::command]
fn cancel_search(search_id: String, state: tauri::State<'_, SearchState>) {
    if let Ok(current) = state.0.lock() {
        if let Some(active) = &current.active {
            if active.id == search_id {
                active.cancel.store(true, Ordering::Relaxed);
            }
        }
    }
}

#[tauri::command]
fn clear_search_index(state: tauri::State<'_, SearchState>) -> Result<(), String> {
    if state
        .0
        .lock()
        .map_err(|_| "検索状態を取得できません。")?
        .active
        .is_some()
    {
        return Err("検索中は索引を削除できません。".into());
    }
    docs_search_core::clear_search_index()
}

#[tauri::command]
async fn get_result_context(
    search_id: String,
    result_id: usize,
    range: Option<ContextRange>,
    state: tauri::State<'_, SearchState>,
) -> Result<ResultContext, ContextError> {
    let target = {
        let current = state
            .0
            .lock()
            .map_err(|_| context_error("contextUnavailable", "検索状態を取得できません。"))?;
        current
            .latest
            .as_ref()
            .filter(|session| session.id == search_id)
            .and_then(|session| session.targets.get(&result_id))
            .cloned()
            .ok_or_else(|| {
                context_error("contextUnavailable", "この結果の周辺情報は取得できません。")
            })?
    };
    let context =
        tauri::async_runtime::spawn_blocking(move || context::get_result_context(&target, range))
            .await
            .map_err(|_| {
                context_error(
                    "contextUnavailable",
                    "周辺情報の取得を完了できませんでした。",
                )
            })??;
    let current = state
        .0
        .lock()
        .map_err(|_| context_error("contextUnavailable", "検索状態を取得できません。"))?;
    if !current
        .latest
        .as_ref()
        .is_some_and(|session| session.id == search_id)
    {
        return Err(context_error(
            "contextUnavailable",
            "新しい検索が始まりました。",
        ));
    }
    Ok(context)
}

#[tauri::command]
fn open_result(path: String) -> Result<(), String> {
    let candidate = std::path::Path::new(&path);
    if !candidate.is_file() {
        return Err("元ファイルが見つかりません。移動または削除された可能性があります。".into());
    }
    open::that(candidate).map_err(|_| "元ファイルを開けませんでした。".into())
}

#[tauri::command]
fn validate_folder_paths(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| {
            let candidate = std::path::Path::new(path.trim());
            if candidate.is_absolute() && candidate.is_dir() {
                path
            } else {
                String::new()
            }
        })
        .collect()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PreparedEdit {
    token: String,
    view: EditView,
}

#[tauri::command]
async fn prepare_result_edit(
    search_id: String,
    result_id: usize,
    evidence_index: Option<usize>,
    state: tauri::State<'_, SearchState>,
) -> Result<PreparedEdit, String> {
    let hit = {
        let current = state.0.lock().map_err(|_| "検索状態を取得できません。")?;
        if current.active.is_some() {
            return Err("検索終了後に編集できます。".into());
        }
        let session = current
            .latest
            .as_ref()
            .filter(|s| s.id == search_id)
            .ok_or("新しい検索が始まりました。")?;
        let hit = session
            .hits
            .get(&result_id)
            .ok_or("編集する結果が見つかりません。")?;
        if session.edited_paths.contains(&hit.file_path) {
            return Err("このファイルは保存後の再検索が必要です。".into());
        }
        hit.clone()
    };
    let (draft, view) =
        tauri::async_runtime::spawn_blocking(move || edit::prepare(&hit, evidence_index))
            .await
            .map_err(|_| "編集内容を取得できません。")??;
    let mut current = state.0.lock().map_err(|_| "検索状態を取得できません。")?;
    if current.active.is_some() {
        return Err("新しい検索が始まりました。".into());
    }
    let session = current
        .latest
        .as_mut()
        .filter(|s| s.id == search_id)
        .ok_or("新しい検索が始まりました。")?;
    if session.edited_paths.contains(&view.file_path) {
        return Err("保存後の再検索が必要です。".into());
    }
    let token = format!(
        "{}:{}:{}",
        result_id,
        evidence_index.unwrap_or(usize::MAX),
        NEXT_EDIT.fetch_add(1, Ordering::Relaxed)
    );
    session.drafts.insert(token.clone(), draft);
    Ok(PreparedEdit { token, view })
}

#[tauri::command]
fn save_result_edit(
    search_id: String,
    token: String,
    replacement: String,
    state: tauri::State<'_, SearchState>,
) -> Result<(), String> {
    let mut current = state.0.lock().map_err(|_| "検索状態を取得できません。")?;
    if current.active.is_some() {
        return Err("検索中は保存できません。入力を控え、検索終了後に再検索してください。".into());
    }
    let session = current
        .latest
        .as_mut()
        .filter(|s| s.id == search_id)
        .ok_or("新しい検索が始まりました。編集入力を控えてください。")?;
    let draft = session
        .drafts
        .get(&token)
        .ok_or("編集状態が無効です。入力を控えて再検索してください。")?;
    let path = draft.path().to_string_lossy().into_owned();
    if session.edited_paths.contains(&path) {
        return Err("このファイルは保存後の再検索が必要です。".into());
    }
    draft.save(&replacement)?;
    session.edited_paths.insert(path.clone());
    session
        .drafts
        .retain(|_, draft| draft.path().to_string_lossy() != path);
    let hits = &session.hits;
    session
        .targets
        .retain(|id, _| hits.get(id).is_some_and(|hit| hit.file_path != path));
    Ok(())
}

#[tauri::command]
fn cancel_result_edit(search_id: String, token: String, state: tauri::State<'_, SearchState>) {
    if let Ok(mut current) = state.0.lock() {
        if let Some(session) = current.latest.as_mut().filter(|s| s.id == search_id) {
            session.drafts.remove(&token);
        }
    }
}

#[tauri::command]
fn format_report(report: Report) -> Result<String, String> {
    report::table_text(&report, Format::Tsv)
}

#[tauri::command]
fn preview_batch(
    query_spec: serde_json::Value,
) -> Result<Vec<docs_search_core::batch::TermSummary>, InputError> {
    let batch = docs_search_core::batch::parse(&query_spec)?;
    Ok(batch
        .terms
        .into_iter()
        .map(|term| docs_search_core::batch::TermSummary {
            term_id: term.id,
            term: term.text,
            ..Default::default()
        })
        .collect())
}

#[tauri::command]
async fn save_report(report: Report, format: Format) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = report::file_bytes(&report, format)?;
        let extension = format.extension();
        let Some(path) = rfd::FileDialog::new()
            .add_filter(extension.to_uppercase(), &[extension])
            .set_file_name(format!("docs-search-results.{extension}"))
            .save_file()
        else {
            return Ok(false);
        };
        std::fs::write(path, bytes).map_err(|_| "出力先へ書き込めませんでした。".to_owned())?;
        Ok(true)
    })
    .await
    .map_err(|_| "出力処理を完了できませんでした。".to_owned())?
}

fn main() {
    tauri::Builder::default()
        .manage(SearchState(Mutex::new(SearchSessions::default())))
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            start_search,
            cancel_search,
            clear_search_index,
            get_result_context,
            validate_folder_paths,
            prepare_result_edit,
            save_result_edit,
            cancel_result_edit,
            open_result,
            format_report,
            preview_batch,
            save_report
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) = window.try_state::<SearchState>() {
                    if let Ok(current) = state.0.lock() {
                        if let Some(active) = &current.active {
                            active.cancel.store(true, Ordering::Relaxed);
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("docs-search desktop failed to start");
}
