use js_sys::{Date, Function, Promise};
use serde_json::{json, Value};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

#[wasm_bindgen(inline_js = r#"
const pageSize = 200;
let fileGroups = [];
let filteredGroups = [];
let groupsByPath = new Map();
let issuePaths = new Set();
let availableTypes = new Set();
let filterQuery = '';
let filterExtension = '';
let totalHitCount = 0;
let filteredHitCount = 0;
let shownGroups = 0;
let visibleLimit = pageSize;
let pendingGroups = new Set();
let renderFrame = null;
let copyStatusTimer = null;
function clearCopyStatus() {
  if (copyStatusTimer !== null) clearTimeout(copyStatusTimer);
  copyStatusTimer = null;
  set_text('copy-status', '');
}
function updateExtensionSummary() {
  const inputs = Array.from(document.querySelectorAll('input[name="extension"]'));
  const count = inputs.filter(input => input.checked).length;
  set_text('selected-extension-count', `${count} 種類選択中`);
  document.getElementById('select-all-extensions').disabled = count === inputs.length;
  document.getElementById('clear-all-extensions').disabled = count === 0;
  if (count > 0) set_text('extensions-error', '');
  document.querySelectorAll('.extension-group').forEach(group => {
    const groupInputs = Array.from(group.querySelectorAll('input[name="extension"]'));
    const allSelected = groupInputs.every(input => input.checked);
    const action = group.querySelector('.group-toggle');
    action.textContent = allSelected ? '分類内をすべて解除' : '分類内をすべて選択';
    action.setAttribute('aria-label', `${group.querySelector('summary').textContent}を${allSelected ? 'すべて解除' : 'すべて選択'}`);
  });
}
function setExtensionChecks(scope, checked) {
  scope.querySelectorAll('input[name="extension"]').forEach(input => { input.checked = checked; });
  updateExtensionSummary();
}
export function init_extension_summary() {
  document.querySelectorAll('input[name="extension"]').forEach(input => input.addEventListener('change', updateExtensionSummary));
  document.getElementById('select-all-extensions').addEventListener('click', () => setExtensionChecks(document, true));
  document.getElementById('clear-all-extensions').addEventListener('click', () => setExtensionChecks(document, false));
  document.querySelectorAll('.extension-group').forEach(group => {
    group.querySelector('.group-toggle').addEventListener('click', () => {
      const inputs = Array.from(group.querySelectorAll('input[name="extension"]'));
      setExtensionChecks(group, !inputs.every(input => input.checked));
    });
  });
  updateExtensionSummary();
}
export function show_extension_picker() { document.querySelector('.extension-picker').open = true; }
function refreshFolderLabels(container, label) {
  container.querySelectorAll('.folder-row').forEach((row, index) => {
    row.querySelector('.folder-input').setAttribute('aria-label', `${label} ${index + 1}`);
    row.querySelector('.folder-browse').setAttribute('aria-label', `${label} ${index + 1} を参照`);
    const remove = row.querySelector('.folder-remove');
    if (remove) remove.setAttribute('aria-label', `${label} ${index + 1} を削除`);
  });
}
function addFolderRow(container, label, errorId) {
  const row = document.createElement('div'); row.className = 'folder-row';
  const entry = document.createElement('input'); entry.className = 'folder-input'; entry.type = 'text'; entry.autocomplete = 'off';
  entry.placeholder = 'フォルダーのパス'; entry.setAttribute('aria-describedby', errorId);
  const browse = document.createElement('button'); browse.className = 'folder-browse'; browse.type = 'button'; browse.textContent = '参照…';
  const remove = document.createElement('button'); remove.className = 'folder-remove'; remove.type = 'button'; remove.textContent = '削除';
  row.append(entry, browse, remove);
  container.append(row);
  refreshFolderLabels(container, label);
  entry.focus();
}
export function init_folder_lists() {
  for (const [containerId, addId, label, errorId] of [
    ['root-folders', 'add-root', '検索フォルダー', 'root-error'],
    ['excluded-folders', 'add-excluded', '対象外フォルダー', 'excluded-error']
  ]) {
    const container = document.getElementById(containerId);
    refreshFolderLabels(container, label);
    document.getElementById(addId).addEventListener('click', () => addFolderRow(container, label, errorId));
    container.addEventListener('input', () => set_text(errorId, ''));
    container.addEventListener('click', async event => {
      const button = event.target.closest('button');
      if (!button) return;
      if (button.classList.contains('folder-remove')) {
        button.closest('.folder-row').remove();
        refreshFolderLabels(container, label);
        set_text(errorId, '');
      } else if (button.classList.contains('folder-browse')) {
        try {
          const path = await window.__TAURI__.core.invoke('pick_folder');
          if (path) {
            button.closest('.folder-row').querySelector('.folder-input').value = path;
            set_text(errorId, '');
            set_text('general-error', '');
          }
        } catch {
          set_text('general-error', 'フォルダーを選択できませんでした。パスを入力してください。');
        }
      }
    });
  }
}
export function selected_folders(id) {
  return Array.from(document.querySelectorAll(`#${id} .folder-input`), entry => entry.value.trim());
}
function renderPreview(target, hit) {
  const characters = Array.from(hit.previewText);
  let cursor = 0;
  for (const [start, end] of hit.matchRanges ?? []) {
    if (!Number.isInteger(start) || !Number.isInteger(end) || start < cursor || end <= start || end > characters.length) continue;
    target.append(document.createTextNode(characters.slice(cursor, start).join('')));
    const mark = document.createElement('mark');
    mark.textContent = characters.slice(start, end).join('');
    target.append(mark);
    cursor = end;
  }
  target.append(document.createTextNode(characters.slice(cursor).join('')));
}
function open_file(path) {
  set_text('general-error', '');
  clearCopyStatus();
  return window.__TAURI__.core.invoke('open_result', { path }).catch(error => {
    set_text('general-error', error?.message ?? String(error));
  });
}
function markGroupError(group) {
  group.hasIssue = true;
  if (!group.row) return;
  group.row.classList.add('has-error');
  if (!group.errorBadge) {
    group.errorBadge = document.createElement('span');
    group.errorBadge.className = 'error-badge';
    group.errorBadge.textContent = 'エラーあり';
    group.heading.append(group.errorBadge);
  }
}
function normalizeFilterText(value) { return value.normalize('NFC').toLocaleLowerCase(); }
function matchesFilter(group) {
  return (!filterExtension || group.fileType.toLowerCase() === filterExtension)
    && (!filterQuery || normalizeFilterText(group.path).includes(filterQuery));
}
function updateFilterSummary() {
  set_text('result-filter-summary', `絞り込み後 ${filteredGroups.length} / ${fileGroups.length} ファイル · ${filteredHitCount} / ${totalHitCount} 件`);
  document.getElementById('result-filter-clear').disabled = !filterQuery && !filterExtension;
}
function addFilterType(fileType) {
  const type = fileType.toLowerCase();
  if (availableTypes.has(type)) return;
  availableTypes.add(type);
  const select = document.getElementById('result-filter-extension');
  const option = document.createElement('option'); option.value = type; option.textContent = `.${type}`;
  const next = Array.from(select.options).slice(1).find(item => item.value.localeCompare(type) > 0);
  select.insertBefore(option, next || null);
}
function applyResultFilter() {
  filterQuery = normalizeFilterText(document.getElementById('result-filter-text').value.trim());
  filterExtension = document.getElementById('result-filter-extension').value;
  if (renderFrame !== null) cancelAnimationFrame(renderFrame);
  renderFrame = null;
  document.getElementById('results').replaceChildren();
  filteredGroups = [];
  filteredHitCount = 0;
  for (const group of fileGroups) {
    group.row = null; group.heading = null; group.count = null; group.matches = null; group.errorBadge = null; group.rendered = 0;
    group.inFilter = matchesFilter(group);
    group.filteredIndex = group.inFilter ? filteredGroups.length : -1;
    if (group.inFilter) { filteredGroups.push(group); filteredHitCount += group.hits.length; }
  }
  shownGroups = 0; visibleLimit = pageSize; pendingGroups = new Set();
  if (totalHitCount > 0) set_text('empty-state', filteredGroups.length === 0
    ? '現在の結果には絞り込みに一致するファイルがありません。' : '結果を表示しています…');
  updateFilterSummary();
  updateMore();
  if (filteredGroups.length > 0) scheduleRender();
}
export function init_result_filter() {
  document.getElementById('result-filter-text').addEventListener('input', applyResultFilter);
  document.getElementById('result-filter-extension').addEventListener('change', applyResultFilter);
  document.getElementById('result-filter-clear').addEventListener('click', () => {
    document.getElementById('result-filter-text').value = '';
    document.getElementById('result-filter-extension').value = '';
    applyResultFilter();
  });
}
function createGroup(group) {
  const row = document.createElement('div'); row.className = 'result'; row.setAttribute('role', 'listitem');
  const heading = document.createElement('div'); heading.className = 'result-heading';
  const type = document.createElement('span'); type.className = 'file-type'; type.dataset.type = group.fileType.toLowerCase(); type.textContent = `.${group.fileType.toLowerCase()}`;
  const name = document.createElement('a'); name.className = 'file-name'; name.href = '#'; name.textContent = group.path.split(/[\\/]/).pop() || group.path;
  const count = document.createElement('span'); count.className = 'match-count'; count.textContent = `${group.hits.length} 件`;
  heading.append(type, name, count);
  const path = document.createElement('span'); path.className = 'path'; path.textContent = group.path;
  const matches = document.createElement('div'); matches.className = 'matches'; matches.setAttribute('role', 'list');
  row.append(heading, path, matches);
  name.addEventListener('click', event => {
    event.preventDefault();
    void open_file(group.path);
  });
  group.row = row;
  group.heading = heading;
  group.count = count;
  group.matches = matches;
  if (group.hasIssue) markGroupError(group);
  return row;
}
function createMatch(group, { hit, location }) {
  const item = document.createElement('div'); item.className = 'match'; item.setAttribute('role', 'listitem');
  const heading = document.createElement('div'); heading.className = 'match-heading';
  const place = document.createElement('span'); place.className = 'location'; place.textContent = location;
  const copy = document.createElement('button'); copy.type = 'button'; copy.className = 'copy-location';
  copy.title = '場所をコピー'; copy.setAttribute('aria-label', `${group.path.split(/[\\/]/).pop() || group.path}、${location}の場所をコピー`);
  copy.innerHTML = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/></svg>';
  heading.append(place, copy);
  const preview = document.createElement('span'); preview.className = 'preview'; renderPreview(preview, hit);
  item.append(heading, preview);
  copy.addEventListener('click', async () => {
    set_text('general-error', '');
    clearCopyStatus();
    try {
      await navigator.clipboard.writeText(`${group.path}\n${location}`);
      set_text('copy-status', '場所をコピーしました。');
      copyStatusTimer = setTimeout(clearCopyStatus, 3500);
    } catch {
      set_text('general-error', 'コピーできませんでした。結果に表示されたパスと場所を確認してください。');
    }
  });
  return item;
}
function updateMore() {
  const remaining = Math.max(0, filteredGroups.length - visibleLimit);
  const more = document.getElementById('more');
  more.hidden = remaining === 0;
  more.textContent = `さらに表示 (${remaining} ファイル)`;
}
function scheduleRender() {
  if (renderFrame === null) renderFrame = requestAnimationFrame(renderPending);
}
function renderPending() {
  renderFrame = null;
  const fragment = document.createDocumentFragment();
  const until = Math.min(visibleLimit, filteredGroups.length);
  for (; shownGroups < until; shownGroups++) {
    const group = filteredGroups[shownGroups];
    fragment.append(createGroup(group));
    pendingGroups.add(group);
  }
  document.getElementById('results').append(fragment);
  let budget = pageSize;
  for (const group of Array.from(pendingGroups)) {
    const matches = document.createDocumentFragment();
    while (group.rendered < group.hits.length && budget > 0) {
      matches.append(createMatch(group, group.hits[group.rendered++]));
      budget--;
    }
    group.matches.append(matches);
    pendingGroups.delete(group);
    if (group.rendered < group.hits.length) pendingGroups.add(group);
    if (budget === 0) break;
  }
  updateMore();
  if (pendingGroups.size > 0 || shownGroups < Math.min(visibleLimit, filteredGroups.length)) scheduleRender();
}
export function bind(id, event, callback) {
  document.getElementById(id).addEventListener(event, event === 'submit' ? e => { e.preventDefault(); callback(); } : callback);
}
export function listen(callback) { return window.__TAURI__.event.listen('search-events', e => { for (const item of e.payload) callback(item); }); }
export function invoke(command, args) { return window.__TAURI__.core.invoke(command, args); }
export function input(id) { return document.getElementById(id).value; }
export function selected_extensions() { return Array.from(document.querySelectorAll('input[name="extension"]:checked'), input => input.value); }
export function set_text(id, value) { document.getElementById(id).textContent = value; }
export function set_status(value) {
  set_text('status', value);
  document.querySelector('.summary').dataset.state = value.includes('失敗') || value.includes('エラー') || value.includes('確認')
    ? 'error' : value.includes('中断') ? 'paused' : value.startsWith('完了') ? 'complete' : 'active';
}
export function disabled(id, value) { document.getElementById(id).disabled = value; }
export function clear_results() {
  clearCopyStatus();
  if (renderFrame !== null) cancelAnimationFrame(renderFrame);
  document.getElementById('results').replaceChildren();
  document.getElementById('issues').replaceChildren();
  const panel = document.getElementById('issues-panel'); panel.hidden = true; panel.open = false;
  set_text('issue-count', '0');
  fileGroups = []; filteredGroups = []; groupsByPath = new Map(); issuePaths = new Set(); availableTypes = new Set();
  filterQuery = ''; filterExtension = ''; totalHitCount = 0; filteredHitCount = 0;
  document.getElementById('result-filter-text').value = '';
  const typeSelect = document.getElementById('result-filter-extension');
  typeSelect.replaceChildren(typeSelect.firstElementChild); typeSelect.value = '';
  shownGroups = 0; visibleLimit = pageSize; pendingGroups = new Set(); renderFrame = null;
  document.getElementById('more').hidden = true;
  updateFilterSummary();
  set_text('empty-state', '一致する場所を探しています…');
}
export function add_hit(hit, location) {
  let group = groupsByPath.get(hit.filePath);
  if (!group) {
    group = { path: hit.filePath, fileType: hit.fileType, hits: [], rendered: 0, hasIssue: issuePaths.has(hit.filePath), row: null, inFilter: false, filteredIndex: -1 };
    groupsByPath.set(hit.filePath, group);
    fileGroups.push(group);
    addFilterType(hit.fileType);
    group.inFilter = matchesFilter(group);
    if (group.inFilter) { group.filteredIndex = filteredGroups.length; filteredGroups.push(group); }
  }
  group.hits.push({ hit, location });
  totalHitCount++;
  if (group.inFilter) {
    filteredHitCount++;
    if (group.count) group.count.textContent = `${group.hits.length} 件`;
    if (group.filteredIndex < visibleLimit) { pendingGroups.add(group); scheduleRender(); }
  }
  if (totalHitCount > 0) set_text('empty-state', filteredGroups.length === 0
    ? '現在の結果には絞り込みに一致するファイルがありません。' : '結果を表示しています…');
  updateFilterSummary();
  updateMore();
}
export function enable_more() { document.getElementById('more').addEventListener('click', () => { visibleLimit += pageSize; updateMore(); scheduleRender(); }); }
export function add_issue(issue) {
  const row = document.createElement('li'); row.textContent = `${issue.path || '(場所不明)'} · ${issue.reason}`; row.title = issue.path || '';
  const list = document.getElementById('issues'); list.append(row);
  if (issue.path) {
    issuePaths.add(issue.path);
    const group = groupsByPath.get(issue.path);
    if (group) markGroupError(group);
  }
  document.getElementById('issues-panel').hidden = false;
  set_text('issue-count', String(list.childElementCount));
}
"#)]
extern "C" {
    fn bind(id: &str, event: &str, callback: &Function);
    fn listen(callback: &Function) -> Promise;
    fn invoke(command: &str, args: JsValue) -> Promise;
    fn input(id: &str) -> String;
    fn selected_extensions() -> JsValue;
    fn selected_folders(id: &str) -> JsValue;
    fn set_text(id: &str, value: &str);
    fn set_status(value: &str);
    fn disabled(id: &str, value: bool);
    fn clear_results();
    fn add_hit(hit: JsValue, location: &str);
    fn enable_more();
    fn init_result_filter();
    fn init_extension_summary();
    fn init_folder_lists();
    fn show_extension_picker();
    fn add_issue(issue: JsValue);
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
    let roots: Vec<String> = serde_wasm_bindgen::from_value(selected_folders("root-folders"))
        .unwrap_or_default();
    let excluded: Vec<String> = serde_wasm_bindgen::from_value(selected_folders("excluded-folders"))
        .unwrap_or_default();
    let root = roots.first().cloned().unwrap_or_default();
    let additional: Vec<String> = roots.into_iter().skip(1).collect();
    let query = input("query");
    let extensions: Vec<String> =
        serde_wasm_bindgen::from_value(selected_extensions()).unwrap_or_default();
    set_text("root-error", "");
    set_text("excluded-error", "");
    set_text("query-error", "");
    set_text("extensions-error", "");
    set_text("general-error", "");
    set_text("copy-status", "");
    if query.trim().is_empty() {
        set_text("query-error", "検索語を入力してください。");
        return;
    }
    if root.is_empty() || additional.iter().any(|directory| directory.is_empty()) {
        set_text("root-error", "検索フォルダーの空欄を入力するか削除してください。");
        return;
    }
    if excluded.iter().any(|directory| directory.is_empty()) {
        set_text("excluded-error", "対象外フォルダーの空欄を入力するか削除してください。");
        return;
    }
    if extensions.is_empty() {
        show_extension_picker();
        set_text(
            "extensions-error",
            "検索する拡張子を1つ以上選んでください。",
        );
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
    set_status("列挙中…");
    set_text("counts", "結果 0 · 処理 0 · エラー 0");
    disabled("search", true);
    disabled("cancel", false);
    let args = json!({"request": {"rootDirectory": root, "additionalDirectories": additional, "excludedDirectories": excluded, "query": query, "recursive": true, "extensions": extensions}, "searchId": id});
    spawn_local(async move {
        if let Err(error) = JsFuture::from(invoke("start_search", js(&args))).await {
            let data = value(error.clone());
            let field = data
                .get("field")
                .and_then(Value::as_str)
                .unwrap_or("general");
            let target = match field {
                "rootDirectory" | "additionalDirectories" => "root-error",
                "excludedDirectories" => "excluded-error",
                "query" => "query-error",
                "extensions" => "extensions-error",
                _ => "general-error",
            };
            set_text(target, &message(error));
            set_status("入力を確認してください。");
            set_text("empty-state", "検索を開始できませんでした。入力とエラーを確認してください。");
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
        set_status("中断中…");
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
        "started" => set_status("列挙中…"),
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
                set_status(
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
        set_status(
            match reason {
                "completed" if issues > 0 => "完了（一部エラーあり）",
                "completed" if results == 0 => "完了 · 該当なし",
                "completed" => "完了",
                "cancelled" => "中断",
                _ => "全体失敗",
            },
        );
        if results == 0 {
            set_text(
                "empty-state",
                match reason {
                    "completed" => "一致する結果はありませんでした。検索語や対象ファイルを確認してください。",
                    "cancelled" => "検索は中断されました。条件を変えて再検索できます。",
                    _ => "検索を完了できませんでした。上のエラーを確認してください。",
                },
            );
        }
        STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            state.running = false;
            state.cancelling = false;
        });
        disabled("search", false);
        disabled("cancel", true);
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    enable_more();
    init_result_filter();
    init_extension_summary();
    init_folder_lists();
    let search = Closure::<dyn FnMut()>::new(on_search);
    bind("search-form", "submit", search.as_ref().unchecked_ref());
    search.forget();
    let cancel = Closure::<dyn FnMut()>::new(on_cancel);
    bind("cancel", "click", cancel.as_ref().unchecked_ref());
    cancel.forget();
    let event = Closure::<dyn FnMut(JsValue)>::new(on_event);
    let _ = listen(event.as_ref().unchecked_ref());
    event.forget();
}
