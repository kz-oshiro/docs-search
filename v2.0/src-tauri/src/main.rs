#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use doc_search_core::{run_search, validate, EventKind, InputError, SearchRequest};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

struct ActiveSearch {
    id: String,
    cancel: Arc<AtomicBool>,
}
struct SearchState(Mutex<Option<ActiveSearch>>);

#[tauri::command]
async fn pick_folder() -> Option<String> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
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
    if current.is_some() {
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
    *current = Some(ActiveSearch {
        id: search_id.clone(),
        cancel: cancel.clone(),
    });
    let id = search_id.clone();
    std::thread::spawn(move || {
        let app_for_events = app.clone();
        let mut batch = Vec::with_capacity(128);
        let _ = run_search(request, id.clone(), &cancel, |event| {
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
                if current.as_ref().is_some_and(|s| s.id == id) {
                    *current = None;
                }
            }
        }
    });
    Ok(search_id)
}

#[tauri::command]
fn cancel_search(search_id: String, state: tauri::State<'_, SearchState>) {
    if let Ok(current) = state.0.lock() {
        if let Some(active) = &*current {
            if active.id == search_id {
                active.cancel.store(true, Ordering::Relaxed);
            }
        }
    }
}

#[tauri::command]
fn open_result(path: String) -> Result<(), String> {
    let candidate = std::path::Path::new(&path);
    if !candidate.is_file() {
        return Err("元ファイルが見つかりません。移動または削除された可能性があります。".into());
    }
    open::that(candidate).map_err(|_| "元ファイルを開けませんでした。".into())
}

fn main() {
    tauri::Builder::default()
        .manage(SearchState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            start_search,
            cancel_search,
            open_result
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) = window.try_state::<SearchState>() {
                    if let Ok(current) = state.0.lock() {
                        if let Some(active) = &*current {
                            active.cancel.store(true, Ordering::Relaxed);
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("doc-search desktop failed to start");
}
