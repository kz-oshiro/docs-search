use js_sys::{Date, Function, Promise};
use serde_json::{json, Value};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

#[wasm_bindgen(inline_js = r#"
let selected = null;
let allHits = [];
let shown = 0;
let scheduled = false;
function renderMore() {
  scheduled = false;
  const until = Math.min(shown + 200, allHits.length);
  const target = document.getElementById('results');
  const fragment = document.createDocumentFragment();
  for (; shown < until; shown++) {
    const { hit, location } = allHits[shown];
    const row = document.createElement('button'); row.type = 'button'; row.className = 'result';
    const line = document.createElement('span'); line.className = 'result-line'; line.textContent = `${hit.fileType.toUpperCase()} · ${location}`;
    const preview = document.createElement('span'); preview.className = 'preview'; preview.textContent = hit.previewText;
    const path = document.createElement('span'); path.className = 'path'; path.textContent = hit.filePath;
    row.append(line, preview, path); row.title = `${hit.filePath}\n${location}`;
    row.addEventListener('click', () => { document.querySelectorAll('.result.selected').forEach(el => el.classList.remove('selected')); row.classList.add('selected'); selected = hit; disabled('open', false); disabled('copy', false); set_text('selection', `${hit.filePath}\n${location}`); });
    fragment.append(row);
  }
  target.append(fragment);
  const more = document.getElementById('more');
  more.hidden = shown >= allHits.length;
  more.textContent = `さらに表示 (${allHits.length - shown} 件)`;
}
export function bind(id, event, callback) {
  document.getElementById(id).addEventListener(event, event === 'submit' ? e => { e.preventDefault(); callback(); } : callback);
}
export function listen(callback) { return window.__TAURI__.event.listen('search-events', e => { for (const item of e.payload) callback(item); }); }
export function invoke(command, args) { return window.__TAURI__.core.invoke(command, args); }
export function input(id) { return document.getElementById(id).value; }
export function set_input(id, value) { document.getElementById(id).value = value; }
export function set_text(id, value) { document.getElementById(id).textContent = value; }
export function disabled(id, value) { document.getElementById(id).disabled = value; }
export function clear_results() { document.getElementById('results').replaceChildren(); document.getElementById('issues').replaceChildren(); allHits = []; shown = 0; scheduled = false; document.getElementById('more').hidden = true; selected = null; disabled('open', true); disabled('copy', true); }
export function add_hit(hit, location) {
  allHits.push({ hit, location });
  if (shown < 200 && !scheduled) { scheduled = true; requestAnimationFrame(renderMore); }
  else if (shown >= 200) { const more = document.getElementById('more'); more.hidden = false; more.textContent = `さらに表示 (${allHits.length - shown} 件)`; }
}
export function enable_more() { document.getElementById('more').addEventListener('click', renderMore); }
export function add_issue(issue) {
  const row = document.createElement('li'); row.textContent = `${issue.path || '(場所不明)'} · ${issue.reason}`; row.title = issue.path || '';
  document.getElementById('issues').append(row);
}
export function selected_hit() { return selected; }
export function copy_text(text) { return navigator.clipboard.writeText(text); }
"#)]
extern "C" {
    fn bind(id: &str, event: &str, callback: &Function);
    fn listen(callback: &Function) -> Promise;
    fn invoke(command: &str, args: JsValue) -> Promise;
    fn input(id: &str) -> String;
    fn set_input(id: &str, value: &str);
    fn set_text(id: &str, value: &str);
    fn disabled(id: &str, value: bool);
    fn clear_results();
    fn add_hit(hit: JsValue, location: &str);
    fn enable_more();
    fn add_issue(issue: JsValue);
    fn selected_hit() -> JsValue;
    fn copy_text(value: &str) -> Promise;
}

#[derive(Default)]
struct State {
    current_id: Option<String>,
    running: bool,
    cancelling: bool,
    next: u64,
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

fn js(value: &Value) -> JsValue {
    js_sys::JSON::parse(&value.to_string()).unwrap_or(JsValue::NULL)
}
fn value(value: JsValue) -> Value {
    serde_wasm_bindgen::from_value(value).unwrap_or(Value::Null)
}
fn message(error: JsValue) -> String {
    let data = value(error);
    data.get("message")
        .and_then(Value::as_str)
        .or_else(|| data.as_str())
        .unwrap_or("操作に失敗しました。")
        .to_owned()
}

fn location(kind: &str, data: &Value) -> String {
    let str_at = |key: &str| {
        data.get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let num_at = |key: &str| data.get(key).and_then(Value::as_u64).unwrap_or(0);
    match kind {
        "fileName" => "ファイル名".into(),
        "cell" => format!("{}!{}", str_at("sheetName"), str_at("cellAddress")),
        "shape" if data.get("slideNumber").is_some() => format!(
            "スライド {} / {}",
            num_at("slideNumber"),
            str_at("shapeName")
        ),
        "shape" => format!(
            "{} / {}{}",
            str_at("sheetName"),
            str_at("shapeName"),
            data.get("anchor")
                .and_then(Value::as_str)
                .map(|a| format!(" ({a})"))
                .unwrap_or_default()
        ),
        "slideTableCell" => format!(
            "スライド {} / {} / 行 {} / 列 {}",
            num_at("slideNumber"),
            str_at("tableName"),
            num_at("row"),
            num_at("column")
        ),
        "paragraph" => format!("本文 段落 {}", num_at("paragraphNumber")),
        "wordTableParagraph" => {
            let mut segments = Vec::new();
            if let Some(path) = data.get("tablePath").and_then(Value::as_array) {
                for item in path {
                    segments.push(format!(
                        "表 {} / 行 {} / 列 {}",
                        item["table"].as_u64().unwrap_or(0),
                        item["row"].as_u64().unwrap_or(0),
                        item["column"].as_u64().unwrap_or(0)
                    ));
                }
            }
            format!(
                "{} / 段落 {}",
                segments.join(" / "),
                num_at("paragraphNumber")
            )
        }
        "textLine" => format!("行 {}", num_at("lineNumber")),
        _ => "場所不明".into(),
    }
}

fn on_search() {
    let root = input("root");
    let query = input("query");
    set_text("root-error", "");
    set_text("query-error", "");
    set_text("general-error", "");
    if query.trim().is_empty() {
        set_text("query-error", "検索語を入力してください。");
        return;
    }
    if root.trim().is_empty() {
        set_text("root-error", "フォルダーを指定してください。");
        return;
    }
    let id = STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        if state.running {
            return None;
        }
        state.next += 1;
        state.running = true;
        state.cancelling = false;
        let id = format!("{}-{}", Date::now() as u64, state.next);
        state.current_id = Some(id.clone());
        Some(id)
    });
    let Some(id) = id else { return };
    clear_results();
    set_text("selection", "結果を選択してください。");
    set_text("status", "列挙中…");
    set_text("counts", "結果 0 · 処理 0 · エラー 0");
    disabled("search", true);
    disabled("cancel", false);
    let args = json!({"request": {"rootDirectory": root, "query": query, "recursive": true}, "searchId": id});
    spawn_local(async move {
        if let Err(error) = JsFuture::from(invoke("start_search", js(&args))).await {
            let data = value(error.clone());
            let field = data
                .get("field")
                .and_then(Value::as_str)
                .unwrap_or("general");
            let target = match field {
                "rootDirectory" => "root-error",
                "query" => "query-error",
                _ => "general-error",
            };
            set_text(target, &message(error));
            set_text("status", "入力を確認してください。");
            STATE.with(|cell| {
                let mut state = cell.borrow_mut();
                state.running = false;
                state.current_id = None;
            });
            disabled("search", false);
            disabled("cancel", true);
        }
    });
}

fn on_cancel() {
    let id = STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        if !state.running || state.cancelling {
            return None;
        }
        state.cancelling = true;
        state.current_id.clone()
    });
    if let Some(id) = id {
        set_text("status", "中断中…");
        disabled("cancel", true);
        spawn_local(async move {
            let _ = JsFuture::from(invoke("cancel_search", js(&json!({"searchId": id})))).await;
        });
    }
}

fn on_event(payload: JsValue) {
    let event = value(payload);
    let id = event.get("searchId").and_then(Value::as_str).unwrap_or("");
    if !STATE.with(|cell| cell.borrow().current_id.as_deref() == Some(id)) {
        return;
    }
    let kind = event.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "started" => set_text("status", "列挙中…"),
        "progress" | "finished" => {
            let counts = &event["counts"];
            let count = |key: &str| counts[key].as_u64().unwrap_or(0);
            let summary = if kind == "progress" && event["phase"] == "discovery" {
                format!(
                    "結果 {} · 発見 {} · 処理 {} · エラー {}",
                    count("resultCount"),
                    count("discoveredFiles"),
                    count("processedFiles"),
                    count("issueCount")
                )
            } else {
                format!(
                    "結果 {} · 処理 {}/{} · エラー {}",
                    count("resultCount"),
                    count("processedFiles"),
                    count("discoveredFiles"),
                    count("issueCount")
                )
            };
            set_text("counts", &summary);
            if kind == "progress" && !STATE.with(|cell| cell.borrow().cancelling) {
                set_text(
                    "status",
                    if event["phase"] == "discovery" {
                        "列挙中…"
                    } else {
                        "検索中…"
                    },
                );
            }
        }
        "result" => {
            let hit = &event["hit"];
            add_hit(
                js(hit),
                &location(hit["sourceKind"].as_str().unwrap_or(""), &hit["location"]),
            );
        }
        "issue" => add_issue(js(&event["issue"])),
        _ => (),
    }
    if kind == "finished" {
        let reason = event["reason"].as_str().unwrap_or("failed");
        let issues = event["counts"]["issueCount"].as_u64().unwrap_or(0);
        let results = event["counts"]["resultCount"].as_u64().unwrap_or(0);
        set_text(
            "status",
            match reason {
                "completed" if issues > 0 => "完了（一部エラーあり）",
                "completed" if results == 0 => "完了 · 該当なし",
                "completed" => "完了",
                "cancelled" => "中断",
                _ => "全体失敗",
            },
        );
        STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            state.running = false;
            state.cancelling = false;
        });
        disabled("search", false);
        disabled("cancel", true);
    }
}

fn on_pick() {
    spawn_local(async {
        if let Ok(path) = JsFuture::from(invoke("pick_folder", js(&json!({})))).await {
            if let Some(path) = path.as_string() {
                set_input("root", &path);
                set_text("root-error", "");
            }
        }
    });
}

fn on_open() {
    let hit = value(selected_hit());
    if let Some(path) = hit
        .get("filePath")
        .and_then(Value::as_str)
        .map(str::to_owned)
    {
        spawn_local(async move {
            if let Err(error) =
                JsFuture::from(invoke("open_result", js(&json!({"path": path})))).await
            {
                set_text("general-error", &message(error));
            }
        });
    }
}

fn on_copy() {
    let hit = value(selected_hit());
    if let Some(path) = hit.get("filePath").and_then(Value::as_str) {
        let text = format!(
            "{}\n{}",
            path,
            location(hit["sourceKind"].as_str().unwrap_or(""), &hit["location"])
        );
        spawn_local(async move {
            match JsFuture::from(copy_text(&text)).await {
                Ok(_) => set_text("general-error", "場所をコピーしました。"),
                Err(_) => set_text(
                    "general-error",
                    "コピーできませんでした。パスは選択欄から確認できます。",
                ),
            }
        });
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    enable_more();
    let search = Closure::<dyn FnMut()>::new(on_search);
    bind("search-form", "submit", search.as_ref().unchecked_ref());
    search.forget();
    let cancel = Closure::<dyn FnMut()>::new(on_cancel);
    bind("cancel", "click", cancel.as_ref().unchecked_ref());
    cancel.forget();
    let pick = Closure::<dyn FnMut()>::new(on_pick);
    bind("pick", "click", pick.as_ref().unchecked_ref());
    pick.forget();
    let open = Closure::<dyn FnMut()>::new(on_open);
    bind("open", "click", open.as_ref().unchecked_ref());
    open.forget();
    let copy = Closure::<dyn FnMut()>::new(on_copy);
    bind("copy", "click", copy.as_ref().unchecked_ref());
    copy.forget();
    let event = Closure::<dyn FnMut(JsValue)>::new(on_event);
    let _ = listen(event.as_ref().unchecked_ref());
    event.forget();
}
