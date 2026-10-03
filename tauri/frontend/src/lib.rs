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
let filterInclude = '';
let filterExclude = '';
let directoryStates = new Map();
let directoryRows = new Map();
let nextResultElement = 0;
let totalHitCount = 0;
let filteredHitCount = 0;
let shownGroups = 0;
let visibleLimit = pageSize;
let pendingGroups = new Set();
let renderFrame = null;
let copyStatusTimer = null;
let reportRequest = null;
let reportFinished = null;
let reportSearchId = '';
let reportIssues = [];
let reportBusy = false;
let fileRankings = new Map();
let rankedPaths = [];
let batchSummary = [];
let executionSummary = null;
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
export function init_search_mode() {
  const mode = document.getElementById('search-mode');
  function update() {
    const advanced = mode.value === 'conditions';
    const batch = mode.value === 'batch';
    document.getElementById('normal-query-field').hidden = advanced || batch;
    document.getElementById('advanced-conditions').hidden = !advanced;
    document.getElementById('batch-input').hidden = !batch;
    document.getElementById('query').disabled = advanced || batch;
    set_text('query-error', ''); set_text('advanced-error', '');
  }
  mode.addEventListener('change', update);
  let revision = 0;
  let previewTimer;
  async function previewBatch(number) {
    const terms = document.getElementById('batch-terms').value.split(/\r?\n/);
    const matchMode = document.getElementById('batch-match-mode').value;
    const invalid = terms.map((text,index) => ({text:text.trim(),line:index+1})).filter(item => item.text && matchMode === 'identifier' && !/^[A-Za-z_][A-Za-z0-9_]*$/.test(item.text));
    if (invalid.length) {
      set_text('batch-count', `入力 ${terms.filter(text=>text.trim()).length} 行`);
      set_text('batch-error', `${invalid.slice(0,5).map(item=>`行${item.line}「${item.text}」`).join('、')}は識別子として扱えません。「通常の文字列検索」に切り替えてください。`);
      return;
    }
    try {
      const entries = await window.__TAURI__.core.invoke('preview_batch', { querySpec: {mode:'batch', terms, matchMode} });
      if (number === revision) { set_text('batch-count', `${entries.length} 語（空行・重複を除去）`); set_text('batch-error',''); }
    } catch (error) { if (number === revision) set_text('batch-error', error?.message ?? String(error)); }
  }
  function scheduleBatchPreview() {
    const number = ++revision;
    window.clearTimeout(previewTimer);
    previewTimer = window.setTimeout(() => { void previewBatch(number); }, 120);
  }
  document.getElementById('batch-terms').addEventListener('input', scheduleBatchPreview);
  document.getElementById('batch-match-mode').addEventListener('change', scheduleBatchPreview);
  update();
}
function refreshFolderLabels(container, label) {
  container.querySelectorAll('.folder-row').forEach((row, index) => {
    row.querySelector('.folder-input').setAttribute('aria-label', `${label} ${index + 1}`);
    row.querySelector('.folder-browse').setAttribute('aria-label', `${label} ${index + 1} を参照`);
    const remove = row.querySelector('.folder-remove');
    if (remove) remove.setAttribute('aria-label', `${label} ${index + 1} を削除`);
  });
}
function addFolderRow(container, label, errorId, focus = true) {
  const row = document.createElement('div'); row.className = 'folder-row';
  const entry = document.createElement('input'); entry.className = 'folder-input'; entry.type = 'text'; entry.autocomplete = 'off';
  entry.placeholder = 'フォルダーのパス'; entry.setAttribute('aria-describedby', errorId);
  const browse = document.createElement('button'); browse.className = 'folder-browse'; browse.type = 'button'; browse.textContent = '参照…';
  const remove = document.createElement('button'); remove.className = 'folder-remove'; remove.type = 'button'; remove.textContent = '削除';
  row.append(entry, browse, remove);
  container.append(row);
  refreshFolderLabels(container, label);
  if (focus) entry.focus();
  return entry;
}
export function init_folder_lists() {
  const storageKey = 'docs-search.folders.v1';
  let folderRevision = 0;
  function persist() {
    folderRevision++;
    try { localStorage.setItem(storageKey, JSON.stringify({roots:selected_folders('root-folders'),excluded:selected_folders('excluded-folders')})); set_text('folder-save-status','フォルダー入力は次回起動時にも使います。'); }
    catch { set_text('folder-save-status','フォルダー入力を保存できません。この起動中だけ使用します。'); }
  }
  for (const [containerId, addId, label, errorId] of [
    ['root-folders', 'add-root', '検索フォルダー', 'root-error'],
    ['excluded-folders', 'add-excluded', '対象外フォルダー', 'excluded-error']
  ]) {
    const container = document.getElementById(containerId);
    refreshFolderLabels(container, label);
    document.getElementById(addId).addEventListener('click', () => { addFolderRow(container, label, errorId); persist(); });
    container.addEventListener('input', () => { set_text(errorId, ''); persist(); });
    container.addEventListener('click', async event => {
      const button = event.target.closest('button');
      if (!button) return;
      if (button.classList.contains('folder-remove')) {
        button.closest('.folder-row').remove();
        refreshFolderLabels(container, label);
        set_text(errorId, '');
        persist();
      } else if (button.classList.contains('folder-browse')) {
        try {
          const path = await window.__TAURI__.core.invoke('pick_folder');
          if (path) {
            button.closest('.folder-row').querySelector('.folder-input').value = path;
            set_text(errorId, '');
            set_text('general-error', '');
            persist();
          }
        } catch {
          set_text('general-error', 'フォルダーを選択できませんでした。パスを入力してください。');
        }
      }
    });
  }
  async function restore() {
    let saved;
    try { saved=JSON.parse(localStorage.getItem(storageKey) ?? 'null'); }
    catch { set_text('folder-save-status','保存したフォルダー入力を読み取れません。パスを入力してください。'); return; }
    if (!saved || !Array.isArray(saved.roots) || !Array.isArray(saved.excluded) || saved.roots.length < 1 || [...saved.roots,...saved.excluded].some(value=>typeof value!=='string')) return;
    const version=folderRevision;
    document.getElementById('search').disabled=true;
    try {
      const validated=await window.__TAURI__.core.invoke('validate_folder_paths',{paths:[...saved.roots,...saved.excluded]});
      if (version!==folderRevision) return;
      for (const [id,label,errorId,values] of [['root-folders','検索フォルダー','root-error',validated.slice(0,saved.roots.length)],['excluded-folders','対象外フォルダー','excluded-error',validated.slice(saved.roots.length)]]) {
        const container=document.getElementById(id);
        if (id==='root-folders') { const first=container.firstElementChild; container.replaceChildren(first); first.querySelector('.folder-input').value=values[0]; for (const value of values.slice(1)) addFolderRow(container,label,errorId,false).value=value; }
        else { container.replaceChildren(); for (const value of values) addFolderRow(container,label,errorId,false).value=value; }
        refreshFolderLabels(container,label);
      }
      persist();
      if (validated.some((value,index)=>!value && [...saved.roots,...saved.excluded][index].trim())) set_text('folder-save-status','無効になったフォルダー欄を空欄にしました。有効な入力と欄の順序は復元しています。');
    } catch { set_text('folder-save-status','保存したフォルダーを確認できません。パスを入力してください。'); }
    finally { document.getElementById('search').disabled=false; }
  }
  void restore();
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
function matchesBody(hit) {
  const texts = hit.evidence?.length ? hit.evidence.map(evidence => evidence.previewText) : [hit.previewText];
  const body = texts.map(text => normalizeFilterText(text ?? '')).join('\n');
  return (!filterInclude || body.includes(filterInclude)) && (!filterExclude || !body.includes(filterExclude));
}
function compareHits(a, b) {
  const left = a.hit.documentOrder ?? [], right = b.hit.documentOrder ?? [];
  for (let i = 0; i < Math.max(left.length, right.length); i++) {
    const difference = (left[i] ?? 0) - (right[i] ?? 0);
    if (difference) return difference;
  }
  const lexical=(left,right)=>left<right ? -1 : left>right ? 1 : 0;
  return lexical(String(a.hit.termId ?? ''),String(b.hit.termId ?? '')) || lexical(String(a.hit.unitKey),String(b.hit.unitKey)) || a.hit.resultId - b.hit.resultId;
}
function insertHit(items, item) {
  let first = 0, last = items.length;
  while (first < last) { const middle = Math.floor((first + last) / 2); if (compareHits(items[middle], item) <= 0) first = middle + 1; else last = middle; }
  items.splice(first, 0, item);
  return first;
}
function parentDirectory(path) { const boundary = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/')); return boundary < 0 ? path : path.slice(0, boundary + (boundary === 2 && path[1] === ':' ? 1 : 0)); }
function directoryKey(path) { return path.replaceAll('/', '\\').toLocaleLowerCase(); }
function updateDirectoryCounts() {
  const counts = new Map();
  for (const group of filteredGroups) {
    const key = directoryKey(parentDirectory(group.path));
    const count = counts.get(key) ?? {files:0, hits:0, errors:0};
    count.files++; count.hits += group.visibleHits.length; count.errors += group.hasIssue ? 1 : 0; counts.set(key, count);
  }
  for (const [key, row] of directoryRows) {
    const count = counts.get(key) ?? {files:0,hits:0,errors:0};
    row.count.textContent = `${count.files} ファイル · ${count.hits} 件${count.errors ? ` · エラー ${count.errors} ファイル` : ''}`;
  }
}
function updateFilterSummary() {
  set_text('result-filter-summary', `絞り込み後 ${filteredGroups.length} / ${fileGroups.length} ファイル · ${filteredHitCount} / ${totalHitCount} 件`);
  document.getElementById('result-filter-clear').disabled = !filterQuery && !filterExtension && !filterInclude && !filterExclude;
  updateReportSummary();
}
function updateReportSummary() {
  const filtered = document.getElementById('export-scope').value === 'filtered';
  const files = filtered ? filteredGroups.length : fileGroups.length;
  const hits = filtered ? filteredHitCount : totalHitCount;
  set_text('export-summary', `${files} ファイル · ${hits} 件を対象`);
}
function setReportEnabled(enabled) {
  for (const id of ['copy-results', 'save-csv', 'save-tsv', 'save-report', 'save-matrix-csv', 'save-matrix-tsv']) {
    document.getElementById(id).disabled = !enabled;
  }
}
function makeReport() {
  const scope = document.getElementById('export-scope').value;
  const groups = scope === 'filtered' ? filteredGroups : fileGroups;
  return {
    schemaVersion: 1, generatedAt: new Date().toISOString(), searchId: reportSearchId, request: reportRequest,
    finishedReason: reportFinished.reason, counts: reportFinished.counts, scope,
    filterText: scope === 'filtered' ? document.getElementById('result-filter-text').value.trim() : '',
    filterExtension: scope === 'filtered' ? filterExtension : '',
    filterInclude: scope === 'filtered' ? document.getElementById('result-filter-include').value.trim() : '',
    filterExclude: scope === 'filtered' ? document.getElementById('result-filter-exclude').value.trim() : '',
    selectedFileCount: groups.length,
    rows: groups.flatMap(group => (scope === 'filtered' ? group.visibleHits : group.hits).map(({ hit, location }) => ({
      resultId: hit.resultId,
      fileName: group.path.split(/[\\/]/).pop() || group.path,
      filePath: group.path, fileType: group.fileType,
      modifiedAt: group.modifiedAt, documentOrder: hit.documentOrder,
      sourceKind: hit.sourceKind, unitKey: hit.unitKey, partKey: hit.partKey,
      groupKey: hit.groupKey, row: hit.row ?? null, column: hit.column ?? null,
      contentClass: hit.contentClass, anchor: hit.anchor ?? null,
      location, locationData: hit.location,
      previewText: hit.previewText, previewTruncated: hit.previewTruncated,
      matchRanges: hit.matchRanges, matchType: hit.matchType,
      matchCategory: hit.matchCategory, score: hit.score,
      evidence: hit.evidence ?? [], termId: hit.termId ?? null, term: hit.term ?? null
    }))),
    issues: reportIssues, rankings: rankedPaths.map(path=>fileRankings.get(path)).filter(Boolean), batchSummary, executionSummary
  };
}
export function report_started(searchId, request) {
  reportSearchId = searchId;
  reportRequest = request;
  const batch = request.querySpec?.mode === 'batch';
  for (const id of ['save-matrix-csv','save-matrix-tsv']) document.getElementById(id).hidden = !batch;
  document.getElementById('batch-results').hidden = !batch;
  if (batch) set_text('batch-summary-list', '検索中…');
}
export function report_finished(reason, counts) {
  reportFinished = { reason, counts };
  setReportEnabled(Boolean(reportRequest) && !reportBusy);
}
export function init_report_actions() {
  document.getElementById('export-scope').addEventListener('change', updateReportSummary);
  for (const [id, format] of [['copy-results', 'copy'], ['save-csv', 'csv'], ['save-tsv', 'tsv'], ['save-report', 'json'], ['save-matrix-csv','matrixcsv'], ['save-matrix-tsv','matrixtsv']]) {
    document.getElementById(id).addEventListener('click', async () => {
      if (!reportFinished || !reportRequest || reportBusy) return;
      const report = makeReport();
      const searchId = report.searchId;
      reportBusy = true;
      setReportEnabled(false);
      clearCopyStatus();
      set_text('general-error', '');
      try {
        if (format === 'copy') {
          const content = await window.__TAURI__.core.invoke('format_report', { report });
          await navigator.clipboard.writeText(content);
          if (searchId === reportSearchId) set_text('copy-status', `${report.rows.length} 件をTSVでコピーしました。`);
        } else {
          const saved = await window.__TAURI__.core.invoke('save_report', { report, format });
          if (saved && searchId === reportSearchId) set_text('copy-status', `${report.rows.length} 件の${format.toUpperCase()}を保存しました。`);
        }
      } catch (error) {
        if (searchId === reportSearchId) set_text('general-error', error?.message ?? String(error));
      } finally {
        reportBusy = false;
        setReportEnabled(reportFinished !== null);
      }
    });
  }
  updateReportSummary();
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
  filterInclude = normalizeFilterText(document.getElementById('result-filter-include').value.trim());
  filterExclude = normalizeFilterText(document.getElementById('result-filter-exclude').value.trim());
  if (renderFrame !== null) cancelAnimationFrame(renderFrame);
  renderFrame = null;
  document.getElementById('results').replaceChildren();
  directoryRows = new Map();
  filteredGroups = [];
  filteredHitCount = 0;
  for (const group of fileGroups) {
    group.row = null; group.heading = null; group.count = null; group.matches = null; group.errorBadge = null; group.rendered = 0;
    group.visibleHits = group.hits.filter(item => matchesBody(item.hit));
    group.inFilter = matchesFilter(group) && group.visibleHits.length > 0;
    group.filteredIndex = group.inFilter ? filteredGroups.length : -1;
    if (group.inFilter) { filteredGroups.push(group); filteredHitCount += group.visibleHits.length; }
    group.dirty = false;
  }
  shownGroups = 0; visibleLimit = pageSize; pendingGroups = new Set();
  if (totalHitCount > 0) set_text('empty-state', filteredGroups.length === 0
    ? '現在の結果には絞り込みに一致するファイルがありません。' : '結果を表示しています…');
  updateFilterSummary();
  updateMore();
  if (filteredGroups.length > 0) scheduleRender();
}
export function init_result_filter() {
  document.getElementById('result-filter-include').addEventListener('input', applyResultFilter);
  document.getElementById('result-filter-exclude').addEventListener('input', applyResultFilter);
  document.getElementById('result-filter-text').addEventListener('input', applyResultFilter);
  document.getElementById('result-filter-extension').addEventListener('change', applyResultFilter);
  document.getElementById('result-filter-clear').addEventListener('click', () => {
    document.getElementById('result-filter-text').value = '';
    document.getElementById('result-filter-extension').value = '';
    document.getElementById('result-filter-include').value = '';
    document.getElementById('result-filter-exclude').value = '';
    applyResultFilter();
  });
}
function createGroup(group) {
  const row = document.createElement('div'); row.className = 'result'; row.setAttribute('role', 'listitem');
  const heading = document.createElement('div'); heading.className = 'result-heading';
  const type = document.createElement('span'); type.className = 'file-type'; type.dataset.type = group.fileType.toLowerCase(); type.textContent = `.${group.fileType.toLowerCase()}`;
  const name = document.createElement('a'); name.className = 'file-name'; name.href = '#'; name.textContent = group.path.split(/[\\/]/).pop() || group.path;
  const count = document.createElement('span'); count.className = 'match-count'; count.textContent = `${group.visibleHits.length} / ${group.hits.length} 件`;
  const toggle = document.createElement('button'); toggle.type = 'button'; toggle.className = 'result-toggle';
  heading.append(toggle, type, name, count);
  const path = document.createElement('span'); path.className = 'path'; path.textContent = group.path;
  const matches = document.createElement('div'); matches.className = 'matches'; matches.setAttribute('role', 'list');
  const modified = document.createElement('span'); modified.className = 'modified-at';
  const dateFormat=new Intl.DateTimeFormat('ja-JP', {year:'numeric',month:'2-digit',day:'2-digit',hour:'2-digit',minute:'2-digit',second:'2-digit',hourCycle:'h23'});
  modified.textContent = Number.isFinite(group.modifiedAt) ? `更新 ${dateFormat.format(new Date(group.modifiedAt))}（${dateFormat.resolvedOptions().timeZone}）` : '更新日時を取得できません';
  row.append(heading, path, modified, matches);
  if (group.needsResearch) { const stale = document.createElement('strong'); stale.className='edited-notice'; stale.textContent='保存済み · このファイルの結果は再検索が必要です'; row.insertBefore(stale,matches); }
  matches.id = `file-matches-${++nextResultElement}`; toggle.setAttribute('aria-controls', matches.id);
  function updateCollapse() { matches.hidden = group.collapsed; toggle.textContent = group.collapsed ? '展開' : '折りたたむ'; toggle.setAttribute('aria-expanded', String(!group.collapsed)); toggle.setAttribute('aria-label', `${name.textContent}の一致箇所を${group.collapsed ? '展開' : '折りたたむ'}`); }
  toggle.addEventListener('click', () => { group.collapsed = !group.collapsed; updateCollapse(); }); updateCollapse();
  const ranking = fileRankings.get(group.path);
  if (ranking) { const reason = document.createElement('p'); reason.className = 'ranking-reasons'; reason.textContent = ranking.reasons.join(' · '); row.insertBefore(reason, matches); }
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
function directoryHost(group) {
  const path = parentDirectory(group.path), key = directoryKey(path);
  let directory = directoryRows.get(key);
  if (directory) return directory.body;
  const row = document.createElement('div'); row.className = 'directory-result'; row.setAttribute('role','listitem');
  const heading = document.createElement('div'); heading.className = 'directory-heading';
  const toggle = document.createElement('button'); toggle.type = 'button';
  const name = document.createElement('strong'); name.textContent = path;
  const count = document.createElement('span'); count.className='directory-count';
  const body = document.createElement('div'); body.className='directory-files'; body.setAttribute('role','list'); body.id=`directory-files-${++nextResultElement}`;
  toggle.setAttribute('aria-controls',body.id);
  function update() { const collapsed=directoryStates.get(key) ?? false; body.hidden=collapsed; toggle.textContent=collapsed ? '展開' : '折りたたむ'; toggle.setAttribute('aria-expanded',String(!collapsed)); toggle.setAttribute('aria-label',`${path}を${collapsed ? '展開' : '折りたたむ'}`); }
  toggle.addEventListener('click',()=>{directoryStates.set(key,!(directoryStates.get(key) ?? false));update();}); update();
  heading.append(toggle,name,count); row.append(heading,body); document.getElementById('results').append(row);
  directory={row,body,count}; directoryRows.set(key,directory); return body;
}
function excelColumn(number) {
  let name = '';
  for (let value = number; value > 0; value = Math.floor((value - 1) / 26)) {
    name = String.fromCharCode(65 + ((value - 1) % 26)) + name;
  }
  return name;
}
function contextMergeAt(context, row, column) {
  return context.merges.find(merge => merge.firstRow <= row && row <= merge.lastRow
    && merge.firstColumn <= column && column <= merge.lastColumn);
}
function renderExcelContext(host, context, load) {
  const firstColumn = context.firstColumn;
  const lastColumn = firstColumn + context.columnCount - 1;
  const controls = document.createElement('div'); controls.className = 'excel-context-controls';
  const title = document.createElement('strong');
  title.textContent = `${context.sheetName}!${context.focusAddress} ${context.focusKind === 'shape' ? '図形アンカー' : context.focusKind === 'excelRow' ? '一致行' : '一致セル'}の周辺`;
  const previous = document.createElement('button'); previous.type = 'button'; previous.textContent = '← 前の列';
  previous.disabled = firstColumn === 1;
  previous.addEventListener('click', () => void load({ firstRow: context.firstRow,
    firstColumn: Math.max(1, firstColumn - 5), rowCount: context.rowCount, columnCount: 5 }));
  const next = document.createElement('button'); next.type = 'button'; next.textContent = '次の列 →';
  next.disabled = lastColumn === 16384;
  next.addEventListener('click', () => void load({ firstRow: context.firstRow,
    firstColumn: Math.min(16380, firstColumn + 5), rowCount: context.rowCount, columnCount: 5 }));
  const position = document.createElement('span'); position.textContent = `${excelColumn(firstColumn)}〜${excelColumn(lastColumn)} 列`;
  controls.append(title, previous, position, next);
  const scroller = document.createElement('div'); scroller.className = 'excel-context-scroll';
  const table = document.createElement('table'); table.className = 'excel-context-table';
  const caption = document.createElement('caption');
  caption.textContent = '保存済みセル値。空欄は保存済みの値がありません。';
  table.append(caption);
  const layout=context.layout ?? {rows:[],columns:[],cells:[],notes:[]};
  const colgroup=document.createElement('colgroup'); colgroup.append(document.createElement('col'));
  for (let column=firstColumn;column<=lastColumn;column++) {
    const col=document.createElement('col'); const saved=layout.columns.find(item=>item.first<=column && column<=item.last);
    const width=saved?.width ?? layout.defaultColumnWidth;
    if (Number.isFinite(width)) col.style.width=`${Math.max(35,Math.min(450,width*7+5))}px`;
    colgroup.append(col);
  }
  table.append(colgroup); table.classList.add('excel-layout-table');
  const header = document.createElement('tr');
  const corner = document.createElement('th'); corner.scope = 'col'; corner.textContent = '行 / 列'; header.append(corner);
  const hiddenRows = new Set(context.hiddenRows);
  const hiddenColumns = new Set(context.hiddenColumns);
  for (let column = firstColumn; column <= lastColumn; column++) {
    const th = document.createElement('th'); th.scope = 'col';
    th.textContent = excelColumn(column) + (hiddenColumns.has(column) ? ' (非表示)' : '');
    header.append(th);
  }
  const thead = document.createElement('thead'); thead.append(header); table.append(thead);
  const values = new Map(context.cells.map(cell => [`${cell.row}:${cell.column}`, cell]));
  const styles=new Map(layout.cells.map(cell=>[`${cell.row}:${cell.column}`,cell.style]));
  const evidenceAddresses = new Set(context.evidenceAddresses ?? []);
  const body = document.createElement('tbody');
  for (let row = context.firstRow; row < context.firstRow + context.rowCount; row++) {
    const tr = document.createElement('tr');
    const height=layout.rows.find(item=>item.row===row)?.height ?? layout.defaultRowHeight;
    if (Number.isFinite(height)) tr.style.height=`${Math.max(20,Math.min(200,height*4/3))}px`;
    const th = document.createElement('th'); th.scope = 'row';
    th.textContent = String(row) + (hiddenRows.has(row) ? ' (非表示)' : ''); tr.append(th);
    for (let column = firstColumn; column <= lastColumn; column++) {
      const merge=contextMergeAt(context,row,column);
      const mergeRow=merge ? Math.max(merge.firstRow,context.firstRow) : row;
      const mergeColumn=merge ? Math.max(merge.firstColumn,firstColumn) : column;
      if (merge && (row!==mergeRow || column!==mergeColumn)) continue;
      const td = document.createElement('td');
      const address = `${excelColumn(column)}${row}`;
      const cell = values.get(`${row}:${column}`);
      if (cell) { renderPreview(td,{previewText:cell.text,matchRanges:cell.matchRanges ?? []}); if (cell.truncated) td.append(document.createTextNode('…')); }
      else if (merge?.anchorText) td.textContent=merge.anchorText+(merge.anchorTruncated ? '…' : '');
      td.title = `${address}${cell ? ': '+cell.text+(cell.truncated ? '…' : '') : merge?.anchorText ? ': '+merge.anchorText : ''}`;
      const format=styles.get(`${row}:${column}`);
      if (format) {
        if (format.horizontal) td.style.textAlign=format.horizontal;
        if (format.vertical) td.style.verticalAlign=format.vertical==='center' ? 'middle' : format.vertical;
        td.style.whiteSpace=format.wrap ? 'pre-wrap' : 'pre';
        td.style.fontWeight=format.bold ? '700' : '400'; td.style.fontStyle=format.italic ? 'italic' : 'normal';
        td.style.textDecoration=[format.underline ? 'underline' : '',format.strike ? 'line-through' : ''].filter(Boolean).join(' ') || 'none';
        if (format.fontName) td.style.fontFamily=format.fontName;
        if (Number.isFinite(format.fontSize)) td.style.fontSize=`${Math.max(6,Math.min(72,format.fontSize))}pt`;
        if (format.color) td.style.color=format.color; if (format.fill) td.style.backgroundColor=format.fill;
        for (const [index,side] of ['Top','Right','Bottom','Left'].entries()) {if(format.borders?.[index]) td.style[`border${side}`]=format.borders[index];}
      }
      let focus=address===context.focusAddress || evidenceAddresses.has(address);
      if (merge) {
        td.rowSpan=Math.min(merge.lastRow,context.firstRow+context.rowCount-1)-row+1;
        td.colSpan=Math.min(merge.lastColumn,lastColumn)-column+1;
        td.classList.add('excel-context-merged');
        td.title += ` · 結合 ${merge.anchorAddress}:${excelColumn(merge.lastColumn)}${merge.lastRow} · 起点 ${merge.anchorAddress}`;
        for (let mergeR=Math.max(merge.firstRow,context.firstRow);mergeR<=Math.min(merge.lastRow,context.firstRow+context.rowCount-1);mergeR++) {
          for (let mergeC=Math.max(merge.firstColumn,firstColumn);mergeC<=Math.min(merge.lastColumn,lastColumn);mergeC++) {
            const mergedAddress=`${excelColumn(mergeC)}${mergeR}`;
            if(mergedAddress===context.focusAddress || evidenceAddresses.has(mergedAddress)) focus=true;
          }
        }
      }
      if(focus) td.classList.add('excel-context-focus');
      tr.append(td);
    }
    body.append(tr);
  }
  table.append(body); scroller.append(table);
  const notes = document.createElement('div'); notes.className = 'excel-context-notes';
  for (const text of layout.notes) { const note=document.createElement('p');note.textContent=text;notes.append(note); }
  for (const merge of context.merges) {
    const note = document.createElement('p');
    const range = `${merge.anchorAddress}:${excelColumn(merge.lastColumn)}${merge.lastRow}`;
    const anchorOutside = merge.firstRow < context.firstRow || merge.firstColumn < firstColumn;
    note.textContent = `結合 ${range} · 起点 ${merge.anchorAddress}`
      + (anchorOutside && merge.anchorText !== null ? `: ${merge.anchorText}${merge.anchorTruncated ? '…' : ''}` : '');
    notes.append(note);
  }
  host.replaceChildren(controls, scroller, notes);
}
function addExcelContext(item, hit) {
  if (!['xlsx', 'xlsm'].includes(hit.fileType.toLowerCase())) return;
  if (!['cell', 'formula', 'shape', 'excelRow'].includes(hit.sourceKind)) return;
  if (hit.sourceKind === 'shape' && !hit.anchor) {
    const unavailable = document.createElement('span'); unavailable.className = 'excel-context-unavailable';
    unavailable.textContent = '図形のアンカー位置がないため、周辺セルを表示できません。';
    item.append(unavailable);
    return;
  }
  const button = document.createElement('button'); button.type = 'button';
  button.className = 'excel-context-toggle'; button.textContent = '周辺を表示'; button.setAttribute('aria-expanded', 'false');
  const host = document.createElement('div'); host.className = 'excel-context'; host.hidden = true;
  item.append(button, host);
  let requestNumber = 0;
  async function load(range) {
    const searchId = reportSearchId;
    if (!searchId) return;
    const number = ++requestNumber;
    host.hidden = false;
    host.textContent = '周辺セルを読み込んでいます…';
    button.textContent = '周辺を閉じる'; button.setAttribute('aria-expanded', 'true');
    try {
      const context = await window.__TAURI__.core.invoke('get_result_context',
        { searchId, resultId: hit.resultId, range });
      if (number !== requestNumber || searchId !== reportSearchId || !item.isConnected) return;
      renderExcelContext(host, context, load);
    } catch (error) {
      if (number !== requestNumber || searchId !== reportSearchId || !item.isConnected) return;
      host.textContent = error?.message ?? String(error);
      host.classList.add('excel-context-error');
    }
  }
  button.addEventListener('click', () => {
    if (!host.hidden) {
      requestNumber++;
      host.hidden = true;
      button.textContent = '周辺を表示'; button.setAttribute('aria-expanded', 'false');
      return;
    }
    host.classList.remove('excel-context-error');
    void load(null);
  });
}
let activeEdit = null;
function addEditAction(item, group, hit) {
  const supported=hit.evidence?.length ? hit.evidence.some(evidence=>evidence.editAnchor) : Boolean(hit.editAnchor);
  if (!supported || group.needsResearch) {
    const reason=document.createElement('small'); reason.className='edit-unavailable';
    reason.textContent=group.needsResearch ? '保存済みのため再検索してから編集できます。' : hit.sourceKind==='fileName' ? 'ファイル名一致は直接編集できません。' : 'Office形式・位置を特定できない結果は元ファイルを開いて編集してください。';
    item.append(reason); return;
  }
  const button=document.createElement('button'); button.type='button'; button.className='edit-result'; button.textContent='一致箇所を編集';
  button.addEventListener('click',()=>{
    if (group.needsResearch) { set_text('general-error','このファイルは保存後の再検索が必要です。'); return; }
    if (!reportFinished) { set_text('general-error','検索終了後に編集できます。'); return; }
    if (activeEdit) { document.getElementById('result-editor').showModal(); return; }
    const dialog=document.getElementById('result-editor');
    const select=document.getElementById('edit-evidence'); select.replaceChildren();
    const candidates=hit.evidence?.length ? hit.evidence.map((evidence,index)=>({evidence,index})).filter(item=>item.evidence.editAnchor) : [{evidence:hit,index:null}];
    for (const {evidence,index} of candidates) { const option=document.createElement('option'); option.value=index===null ? '' : String(index); option.textContent=`${evidence.term ? evidence.term+' · ' : ''}${evidence.locationText ?? locationForEdit(evidence)} · 一致箇所`; select.append(option); }
    select.disabled=candidates.length===1;
    activeEdit={searchId:reportSearchId,hit,group,token:null,button,busy:false};
    document.getElementById('edit-replacement').value='';
    document.getElementById('edit-save').disabled=true;
    set_text('edit-error',''); set_text('edit-location',group.path); set_text('edit-context','編集する根拠を選び、「読み込む」で元の一致箇所を取得してください。');
    dialog.showModal();
  }); item.append(button);
}
function locationForEdit(hit) { return hit.location?.lineNumber ? `行 ${hit.location.lineNumber}` : '選択した箇所'; }
export function init_edit_actions() {
  const dialog=document.getElementById('result-editor');
  async function cancelEdit() {
    if (!activeEdit || activeEdit.busy) return;
    const edit=activeEdit; activeEdit=null; dialog.close();
    document.getElementById('search').disabled=false;
    if (edit.token) await window.__TAURI__.core.invoke('cancel_result_edit',{searchId:edit.searchId,token:edit.token}).catch(()=>{});
    if (edit.button.isConnected) edit.button.focus();
  }
  dialog.addEventListener('cancel',event=>{event.preventDefault();void cancelEdit();});
  document.getElementById('edit-cancel').addEventListener('click',()=>void cancelEdit());
  document.getElementById('edit-evidence').addEventListener('change',()=>{document.getElementById('edit-save').disabled=true;set_text('edit-error','根拠を切り替えるには読み込みを選んでください。編集中の文字列は読み込むまで保持しています。');});
  document.getElementById('edit-load').addEventListener('click',async()=>{
    const edit=activeEdit; if (!edit || edit.busy) return;
    edit.busy=true; document.getElementById('edit-load').disabled=true; document.getElementById('edit-save').disabled=true; document.getElementById('edit-evidence').disabled=true;
    set_text('edit-error','');
    try {
      if (edit.token) await window.__TAURI__.core.invoke('cancel_result_edit',{searchId:edit.searchId,token:edit.token});
      edit.token=null;
      const value=document.getElementById('edit-evidence').value;
      const prepared=await window.__TAURI__.core.invoke('prepare_result_edit',{searchId:edit.searchId,resultId:edit.hit.resultId,evidenceIndex:value==='' ? null : Number(value)});
      if (activeEdit!==edit) return;
      edit.token=prepared.token;
      set_text('edit-location',`${prepared.view.filePath} · 行 ${prepared.view.lineNumber} · ${prepared.view.encoding}`);
      set_text('edit-context',`${prepared.view.before}【${prepared.view.selectedText}】${prepared.view.after}`);
      document.getElementById('edit-replacement').value=prepared.view.selectedText;
      document.getElementById('edit-save').disabled=false; document.getElementById('edit-replacement').focus();
      document.getElementById('search').disabled=true;
    } catch (error) { set_text('edit-error',error?.message ?? String(error)); }
    finally { edit.busy=false; document.getElementById('edit-load').disabled=false; document.getElementById('edit-evidence').disabled=document.getElementById('edit-evidence').options.length===1; }
  });
  document.getElementById('edit-save').addEventListener('click',async()=>{
    const edit=activeEdit; if (!edit?.token || edit.busy) return;
    edit.busy=true; document.getElementById('edit-save').disabled=true; document.getElementById('edit-load').disabled=true; document.getElementById('edit-evidence').disabled=true;
    try {
      await window.__TAURI__.core.invoke('save_result_edit',{searchId:edit.searchId,token:edit.token,replacement:document.getElementById('edit-replacement').value});
      edit.group.needsResearch=true; activeEdit=null; dialog.close(); document.getElementById('search').disabled=false;
      set_text('copy-status','一致箇所を保存しました。このファイルの検索結果は再検索して更新してください。'); applyResultFilter();
    } catch (error) { set_text('edit-error',error?.message ?? String(error)); document.getElementById('edit-save').disabled=false; }
    finally { edit.busy=false; document.getElementById('edit-load').disabled=false; document.getElementById('edit-evidence').disabled=document.getElementById('edit-evidence').options.length===1; }
  });
}
function createMatch(group, { hit, location }) {
  const item = document.createElement('div'); item.className = 'match'; item.setAttribute('role', 'listitem');
  const heading = document.createElement('div'); heading.className = 'match-heading';
  const place = document.createElement('span'); place.className = 'location'; place.textContent = location;
  const copy = document.createElement('button'); copy.type = 'button'; copy.className = 'copy-location';
  copy.title = '場所をコピー'; copy.setAttribute('aria-label', `${group.path.split(/[\\/]/).pop() || group.path}、${location}の場所をコピー`);
  copy.innerHTML = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/></svg>';
  const category = document.createElement('span'); category.className = 'match-category';
  category.dataset.category = ['standard', 'fuzzy'].includes(hit.matchCategory) ? hit.matchCategory : 'unknown';
  category.textContent = { standard: '一致検索', fuzzy: 'あいまい検索' }[hit.matchCategory] || '判定不明';
  const labels = { exact: '完全一致', caseFolded: '表記揺れ', normalized: '表記揺れ', separatorVariant: '表記揺れ', identifier: '識別子一致', kanaVariant: '表記揺れ', prefix: '前方一致', substring: '部分一致', editDistance: 'タイプミス候補' };
  const reason = document.createElement('span'); reason.className = 'match-type'; reason.textContent = labels[hit.matchType] || '一致';
  heading.append(place, category, reason, copy);
  if (hit.term) { const term = document.createElement('strong'); term.textContent = hit.term; heading.prepend(term); }
  item.append(heading);
  if (hit.evidence?.length) {
    const list = document.createElement('ul'); list.className = 'evidence-list';
    for (const evidence of hit.evidence) {
      const row = document.createElement('li');
      const label = document.createElement('strong'); label.className = 'evidence-heading';
      label.textContent = `${evidence.term} · ${evidence.locationText || '場所不明'}`;
      const excerpt = document.createElement('span'); excerpt.className = 'evidence-preview';
      renderPreview(excerpt, evidence);
      row.append(label, excerpt); list.append(row);
    }
    item.append(list);
  } else {
    const preview = document.createElement('span'); preview.className = 'preview'; renderPreview(preview, hit);
    item.append(preview);
  }
  addExcelContext(item, hit);
  addEditAction(item, group, hit);
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
  const until = Math.min(visibleLimit, filteredGroups.length);
  for (; shownGroups < until; shownGroups++) {
    const group = filteredGroups[shownGroups];
    directoryHost(group).append(createGroup(group));
    pendingGroups.add(group);
  }
  updateDirectoryCounts();
  let budget = pageSize;
  for (const group of Array.from(pendingGroups)) {
    if (!group.matches) { pendingGroups.delete(group); continue; }
    if (group.dirty) { group.matches.replaceChildren(); group.rendered=0; group.dirty=false; }
    const matches = document.createDocumentFragment();
    while (group.rendered < group.visibleHits.length && budget > 0) {
      matches.append(createMatch(group, group.visibleHits[group.rendered++]));
      budget--;
    }
    group.matches.append(matches);
    pendingGroups.delete(group);
    if (group.rendered < group.visibleHits.length) pendingGroups.add(group);
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
export function checked(id) { return document.getElementById(id).checked; }
export function selected_extensions() { return Array.from(document.querySelectorAll('input[name="extension"]:checked'), input => input.value); }
export function set_text(id, value) { document.getElementById(id).textContent = value; }
export function set_status(value) {
  set_text('status', value);
  document.querySelector('.summary').dataset.state = value === '全体失敗' ? 'failed'
    : value.includes('エラー') ? 'warning' : value.includes('失敗') || value.includes('確認')
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
  directoryStates = new Map(); directoryRows = new Map();
  reportRequest = null; reportFinished = null; reportSearchId = ''; reportIssues = [];
  fileRankings = new Map(); rankedPaths = []; batchSummary = []; executionSummary = null;
  document.getElementById('batch-results').hidden = true;
  document.getElementById('batch-summary-list').replaceChildren(); document.getElementById('batch-matrix-content').replaceChildren();
  document.getElementById('batch-matrix').ontoggle = null;
  document.getElementById('batch-matrix').open = false;
  setReportEnabled(false);
  filterQuery = ''; filterExtension = ''; filterInclude=''; filterExclude=''; totalHitCount = 0; filteredHitCount = 0;
  document.getElementById('result-filter-include').value = ''; document.getElementById('result-filter-exclude').value = '';
  document.getElementById('result-filter-text').value = '';
  document.getElementById('export-scope').value = 'filtered';
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
    group = { path: hit.filePath, fileType: hit.fileType, modifiedAt:hit.modifiedAt, hits: [], visibleHits:[], collapsed:false, dirty:false, needsResearch:false, rendered: 0, hasIssue: issuePaths.has(hit.filePath), row: null, inFilter: false, filteredIndex: -1 };
    groupsByPath.set(hit.filePath, group);
    fileGroups.push(group);
    addFilterType(hit.fileType);
  }
  const item = {hit,location}; insertHit(group.hits,item);
  totalHitCount++;
  if (matchesFilter(group) && matchesBody(hit)) {
    const position=insertHit(group.visibleHits,item);
    if (!group.inFilter) { group.inFilter=true; group.filteredIndex=filteredGroups.length; filteredGroups.push(group); }
    filteredHitCount++;
    group.dirty = group.dirty || position < group.rendered;
    if (group.count) group.count.textContent = `${group.visibleHits.length} / ${group.hits.length} 件`;
    if (group.filteredIndex < visibleLimit) { pendingGroups.add(group); scheduleRender(); }
    else scheduleRender();
  }
  if (group.count) group.count.textContent = `${group.visibleHits.length} / ${group.hits.length} 件`;
  if (totalHitCount > 0) set_text('empty-state', filteredGroups.length === 0
    ? '現在の結果には絞り込みに一致するファイルがありません。' : '結果を表示しています…');
  updateFilterSummary();
  updateMore();
}
export function file_ranked(ranking) { fileRankings.set(ranking.filePath, ranking); }
export function ranking_summary(files) { rankedPaths = files; }
export function execution_summary(data) { executionSummary = data; }
export function rank_results() {
  const positions = new Map(rankedPaths.map((path,index)=>[path,index]));
  fileGroups.sort((a,b)=>(positions.get(a.path) ?? Number.MAX_SAFE_INTEGER) - (positions.get(b.path) ?? Number.MAX_SAFE_INTEGER));
  for (const group of fileGroups) {
    group.hits.sort(compareHits);
  }
  applyResultFilter();
}
export function batch_summary(terms) {
  batchSummary = terms;
  const host = document.getElementById('batch-summary-list'); host.replaceChildren();
  for (const term of terms) {
    const detail = document.createElement('details');
    const heading = document.createElement('summary'); heading.textContent = `${term.term} · ${term.fileCount} ファイル / ${term.hitCount} 件 · ${term.status}`;
    const list = document.createElement('div'); const more = document.createElement('button'); more.type = 'button'; more.textContent = 'さらに50件を表示';
    let entries = null, shown = 0;
    function render() {
      if (entries === null) entries = rankedPaths.flatMap(path=>(groupsByPath.get(path)?.hits ?? []).filter(item=>item.hit.termId === term.termId).map(item=>({group:groupsByPath.get(path),item})));
      const end = Math.min(shown+50, entries.length);
      for (;shown<end;shown++) {
        const {group,item} = entries[shown]; const card = document.createElement('div'); const name = document.createElement('button'); name.type='button'; name.textContent=group.path; name.addEventListener('click',()=>void open_file(group.path));
        card.append(name, createMatch(group,item)); list.append(card);
      }
      more.hidden = shown === entries.length;
    }
    detail.addEventListener('toggle',()=>{ if (detail.open && entries === null) render(); }); more.addEventListener('click',render);
    detail.append(heading,list,more); more.hidden=true; host.append(detail);
  }
  let termPage=0, filePage=0;
  function renderMatrix() {
    const host=document.getElementById('batch-matrix-content'); host.replaceChildren();
    const table=document.createElement('table'); const head=document.createElement('tr'); const corner=document.createElement('th'); corner.textContent='検索語 / ファイル'; head.append(corner);
    const files=rankedPaths.slice(filePage*20,filePage*20+20);
    for (const path of files) { const th=document.createElement('th'); th.scope='col'; th.textContent=path; head.append(th); } table.append(head);
    for (const term of terms.slice(termPage*20,termPage*20+20)) {
      const row=document.createElement('tr'); const title=document.createElement('th'); title.scope='row'; title.textContent=term.term; row.append(title);
      for (const path of files) { const cell=document.createElement('td'); cell.textContent=String((groupsByPath.get(path)?.hits ?? []).filter(item=>item.hit.termId===term.termId).length); row.append(cell); } table.append(row);
    }
    host.append(table);
    for (const [label,enabled,action] of [['前の語',termPage>0,()=>termPage--],['次の語',(termPage+1)*20<terms.length,()=>termPage++],['前のファイル',filePage>0,()=>filePage--],['次のファイル',(filePage+1)*20<rankedPaths.length,()=>filePage++]]) {
      const button=document.createElement('button'); button.type='button'; button.textContent=label; button.disabled=!enabled; button.addEventListener('click',()=>{action();renderMatrix();}); host.append(button);
    }
  }
  const matrix=document.getElementById('batch-matrix');
  matrix.ontoggle=()=>{if(matrix.open) renderMatrix();};
  if (matrix.open) renderMatrix();
}
export function enable_more() { document.getElementById('more').addEventListener('click', () => { visibleLimit += pageSize; updateMore(); scheduleRender(); }); }
export function add_issue(issue) {
  reportIssues.push(issue);
  const row = document.createElement('li');
  const label = document.createElement('span'); label.className = 'issue-label'; label.textContent = issue.stage === 'read' ? '読み取りエラー' : issue.stage === 'discovery' ? '列挙エラー' : 'エラー';
  const path = document.createElement('strong'); path.className = 'issue-path'; path.textContent = issue.path || '場所不明';
  const reason = document.createElement('span'); reason.className = 'issue-reason'; reason.textContent = issue.reason;
  row.append(label, path, reason);
  const list = document.getElementById('issues'); list.append(row);
  if (issue.path) {
    issuePaths.add(issue.path);
    const group = groupsByPath.get(issue.path);
    if (group) markGroupError(group);
    updateDirectoryCounts();
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
    fn checked(id: &str) -> bool;
    fn selected_extensions() -> JsValue;
    fn selected_folders(id: &str) -> JsValue;
    fn set_text(id: &str, value: &str);
    fn set_status(value: &str);
    fn disabled(id: &str, value: bool);
    fn clear_results();
    fn add_hit(hit: JsValue, location: &str);
    fn rank_results();
    fn file_ranked(ranking: JsValue);
    fn ranking_summary(files: JsValue);
    fn batch_summary(terms: JsValue);
    fn execution_summary(data: JsValue);
    fn enable_more();
    fn init_result_filter();
    fn init_extension_summary();
    fn init_search_mode();
    fn init_folder_lists();
    fn init_edit_actions();
    fn show_extension_picker();
    fn add_issue(issue: JsValue);
    fn report_started(search_id: &str, request: JsValue);
    fn report_finished(reason: &str, counts: JsValue);
    fn init_report_actions();
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
        "formula" => format!(
            "{}!{} / 数式{}",
            str_at("sheetName"),
            str_at("cellAddress"),
            data.get("sharedIndex")
                .and_then(Value::as_str)
                .map(|id| format!("（共有式 {id}）"))
                .unwrap_or_default()
        ),
        "excelComment" => format!(
            "{}!{} / コメント {}",
            str_at("sheetName"),
            str_at("cellAddress"),
            str_at("commentId")
        ),
        "note" => format!(
            "スライド {} / ノート {} / 段落 {}",
            num_at("slideNumber"),
            str_at("shapeName"),
            num_at("paragraphNumber")
        ),
        "wordHeader" | "wordFooter" | "wordComment" => {
            let label = match kind {
                "wordHeader" => "ヘッダー",
                "wordFooter" => "フッター",
                _ => "コメント",
            };
            let place = if data.get("tablePath").is_some() {
                location("wordTableParagraph", data)
            } else {
                format!("段落 {}", num_at("paragraphNumber"))
            };
            format!(
                "{label} {} {} / {place}",
                str_at("partName"),
                if kind == "wordComment" {
                    str_at("commentId")
                } else {
                    str_at("sectionType")
                }
            )
        }
        "excelRow" => format!("{} / 行 {}", str_at("sheetName"), num_at("row")),
        "fileMatch" => "ファイル全体".into(),
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
    let roots: Vec<String> =
        serde_wasm_bindgen::from_value(selected_folders("root-folders")).unwrap_or_default();
    let excluded: Vec<String> =
        serde_wasm_bindgen::from_value(selected_folders("excluded-folders")).unwrap_or_default();
    let root = roots.first().cloned().unwrap_or_default();
    let additional: Vec<String> = roots.into_iter().skip(1).collect();
    let advanced = input("search-mode") == "conditions";
    let batch = input("search-mode") == "batch";
    let query = if advanced || batch {
        String::new()
    } else {
        input("query")
    };
    let query_spec = if batch {
        Some(
            json!({"mode":"batch", "terms":input("batch-terms").lines().map(str::to_owned).collect::<Vec<_>>(), "matchMode":input("batch-match-mode")}),
        )
    } else {
        advanced.then(|| {
            json!({
                "mode": "conditions",
                "scope": input("condition-scope"),
                "all": input("all-terms").lines().map(str::to_owned).collect::<Vec<_>>(),
                "any": input("any-terms").lines().map(str::to_owned).collect::<Vec<_>>(),
                "not": input("not-terms").lines().map(str::to_owned).collect::<Vec<_>>()
            })
        })
    };
    let extensions: Vec<String> =
        serde_wasm_bindgen::from_value(selected_extensions()).unwrap_or_default();
    set_text("root-error", "");
    set_text("excluded-error", "");
    set_text("query-error", "");
    set_text("advanced-error", "");
    set_text("batch-error", "");
    set_text("extensions-error", "");
    set_text("general-error", "");
    set_text("copy-status", "");
    if !advanced && !batch && query.trim().is_empty() {
        set_text("query-error", "検索語を入力してください。");
        return;
    }
    if advanced && input("all-terms").trim().is_empty() && input("any-terms").trim().is_empty() {
        set_text(
            "advanced-error",
            "「すべて含む」か「いずれか含む」に語句を入力してください。",
        );
        return;
    }
    if root.is_empty() || additional.iter().any(|directory| directory.is_empty()) {
        set_text(
            "root-error",
            "検索フォルダーの空欄を入力するか削除してください。",
        );
        return;
    }
    if excluded.iter().any(|directory| directory.is_empty()) {
        set_text(
            "excluded-error",
            "対象外フォルダーの空欄を入力するか削除してください。",
        );
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
    let args = json!({"request": {"rootDirectory": root, "additionalDirectories": additional, "excludedDirectories": excluded, "query": query, "querySpec": query_spec, "recursive": true, "extensions": extensions, "useIndex": checked("use-index"), "fuzzySearch": checked("fuzzy-search"), "includeNotes":checked("include-notes"), "includeFormulas":checked("include-formulas")}, "searchId": id});
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
                "querySpec" if batch => "batch-error",
                "querySpec" => "advanced-error",
                "extensions" => "extensions-error",
                _ => "general-error",
            };
            set_text(target, &message(error));
            set_status("入力を確認してください。");
            set_text(
                "empty-state",
                "検索を開始できませんでした。入力とエラーを確認してください。",
            );
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

fn on_clear_index() {
    set_text("general-error", "");
    spawn_local(async move {
        match JsFuture::from(invoke("clear_search_index", JsValue::NULL)).await {
            Ok(_) => set_text(
                "copy-status",
                "検索用索引を削除しました。次回、索引を有効にすると再作成します。",
            ),
            Err(error) => set_text("general-error", &message(error)),
        }
    });
}

fn on_event(payload: JsValue) {
    let event = value(payload);
    let id = event.get("searchId").and_then(Value::as_str).unwrap_or("");
    if !STATE.with(|cell| cell.borrow().current_id.as_deref() == Some(id)) {
        return;
    }
    let kind = event.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "started" => {
            report_started(id, js(&event["request"]));
            set_status("列挙中…");
        }
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
                set_status(if event["phase"] == "discovery" {
                    "列挙中…"
                } else {
                    "検索中…"
                });
            }
        }
        "result" => {
            let mut hit = event["hit"].clone();
            if let Some(evidence) = hit["evidence"].as_array_mut() {
                for item in evidence {
                    item["locationText"] = json!(location(
                        item["sourceKind"].as_str().unwrap_or(""),
                        &item["location"],
                    ));
                }
            }
            add_hit(
                js(&hit),
                &location(hit["sourceKind"].as_str().unwrap_or(""), &hit["location"]),
            );
        }
        "issue" => add_issue(js(&event["issue"])),
        "fileRanked" => file_ranked(js(&event["ranking"])),
        "rankingSummary" => ranking_summary(js(&event["files"])),
        "batchSummary" => batch_summary(js(&event["terms"])),
        "executionSummary" => execution_summary(js(&event)),
        _ => (),
    }
    if kind == "finished" {
        rank_results();
        let reason = event["reason"].as_str().unwrap_or("failed");
        report_finished(reason, js(&event["counts"]));
        let issues = event["counts"]["issueCount"].as_u64().unwrap_or(0);
        let results = event["counts"]["resultCount"].as_u64().unwrap_or(0);
        set_status(match reason {
            "completed" if issues > 0 => "完了（一部エラーあり）",
            "completed" if results == 0 => "完了 · 該当なし",
            "completed" => "完了",
            "cancelled" => "中断",
            _ => "全体失敗",
        });
        if results == 0 {
            set_text(
                "empty-state",
                match reason {
                    "completed" => {
                        "一致する結果はありませんでした。検索語や対象ファイルを確認してください。"
                    }
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
    let clear = Closure::<dyn FnMut()>::new(on_clear_index);
    bind("clear-index", "click", clear.as_ref().unchecked_ref());
    clear.forget();
    init_result_filter();
    init_report_actions();
    init_extension_summary();
    init_search_mode();
    init_folder_lists();
    init_edit_actions();
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
