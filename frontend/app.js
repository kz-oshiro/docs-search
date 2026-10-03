import * as view from './view.js';
import { invoke, listen } from './tauri.js';

const state = { currentId: null, running: false, cancelling: false, next: 0, ready: false };
let initialization;
const message = error => typeof error?.message === 'string' ? error.message
  : typeof error === 'string' ? error : '操作に失敗しました。';
// Match Rust str::lines(), including CRLF and the absence of a final empty line.
const lines = text => text === '' ? [] : text.replace(/\r\n/g, '\n').replace(/\n$/, '').split('\n');
const trim = text => text.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '');

export function location(kind, data = {}) {
  data ??= {};
  const s = key => typeof data?.[key] === 'string' ? data[key] : '';
  const n = key => Number.isSafeInteger(data?.[key]) && data[key] >= 0 ? data[key] : 0;
  switch (kind) {
    case 'fileName': return 'ファイル名';
    case 'cell': return `${s('sheetName')}!${s('cellAddress')}`;
    case 'formula': return `${s('sheetName')}!${s('cellAddress')} / 数式${typeof data.sharedIndex === 'string' ? `（共有式 ${data.sharedIndex}）` : ''}`;
    case 'excelComment': return `${s('sheetName')}!${s('cellAddress')} / コメント ${s('commentId')}`;
    case 'note': return `スライド ${n('slideNumber')} / ノート ${s('shapeName')} / 段落 ${n('paragraphNumber')}`;
    case 'wordHeader': case 'wordFooter': case 'wordComment': {
      const label = { wordHeader: 'ヘッダー', wordFooter: 'フッター', wordComment: 'コメント' }[kind];
      const place = Object.hasOwn(data, 'tablePath') ? location('wordTableParagraph', data) : `段落 ${n('paragraphNumber')}`;
      return `${label} ${s('partName')} ${s(kind === 'wordComment' ? 'commentId' : 'sectionType')} / ${place}`;
    }
    case 'excelRow': return `${s('sheetName')} / 行 ${n('row')}`;
    case 'fileMatch': return 'ファイル全体';
    case 'shape': return Object.hasOwn(data, 'slideNumber') ? `スライド ${n('slideNumber')} / ${s('shapeName')}`
      : `${s('sheetName')} / ${s('shapeName')}${typeof data.anchor === 'string' ? ` (${data.anchor})` : ''}`;
    case 'slideTableCell': return `スライド ${n('slideNumber')} / ${s('tableName')} / 行 ${n('row')} / 列 ${n('column')}`;
    case 'paragraph': return `本文 段落 ${n('paragraphNumber')}`;
    case 'wordTableParagraph': return `${(Array.isArray(data.tablePath) ? data.tablePath : []).map(item => {
      const number = key => Number.isSafeInteger(item?.[key]) && item[key] >= 0 ? item[key] : 0;
      return `表 ${number('table')} / 行 ${number('row')} / 列 ${number('column')}`;
    }).join(' / ')} / 段落 ${n('paragraphNumber')}`;
    case 'textLine': return `行 ${n('lineNumber')}`;
    default: return '場所不明';
  }
}

async function onSearch() {
  if (!state.ready || state.running) return;
  const roots = view.selected_folders('root-folders');
  const excluded = view.selected_folders('excluded-folders');
  const root = roots[0] || '';
  const additional = roots.slice(1);
  const mode = view.input('search-mode');
  const advanced = mode === 'conditions', batch = mode === 'batch';
  const query = advanced || batch ? '' : view.input('query');
  const querySpec = batch ? { mode: 'batch', terms: lines(view.input('batch-terms')), matchMode: view.input('batch-match-mode') }
    : advanced ? { mode: 'conditions', scope: view.input('condition-scope'), all: lines(view.input('all-terms')),
      any: lines(view.input('any-terms')), not: lines(view.input('not-terms')) } : null;
  const extensions = view.selected_extensions();
  for (const id of ['root-error', 'excluded-error', 'query-error', 'advanced-error', 'batch-error', 'extensions-error', 'general-error', 'copy-status']) view.set_text(id, '');
  if (!advanced && !batch && !trim(query)) return view.set_text('query-error', '検索語を入力してください。');
  if (advanced && !trim(view.input('all-terms')) && !trim(view.input('any-terms')))
    return view.set_text('advanced-error', '「すべて含む」か「いずれか含む」に語句を入力してください。');
  if (!root || additional.some(directory => !directory)) return view.set_text('root-error', '検索フォルダーの空欄を入力するか削除してください。');
  if (excluded.some(directory => !directory)) return view.set_text('excluded-error', '対象外フォルダーの空欄を入力するか削除してください。');
  if (!extensions.length) {
    view.show_extension_picker();
    return view.set_text('extensions-error', '検索する拡張子を1つ以上選んでください。');
  }
  const searchId = `${Date.now()}-${++state.next}`;
  Object.assign(state, { currentId: searchId, running: true, cancelling: false });
  view.clear_results(); view.set_status('列挙中…'); view.set_text('counts', '結果 0 · 処理 0 · エラー 0');
  view.disabled('search', true); view.disabled('cancel', false);
  try {
    await invoke('start_search', { request: { rootDirectory: root, additionalDirectories: additional,
      excludedDirectories: excluded, query, querySpec, recursive: true, extensions,
      useIndex: view.checked('use-index'), fuzzySearch: view.checked('fuzzy-search'),
      includeNotes: view.checked('include-notes'), includeFormulas: view.checked('include-formulas') }, searchId });
  } catch (error) {
    if (state.currentId !== searchId || !state.running) return;
    const target = { rootDirectory: 'root-error', additionalDirectories: 'root-error', excludedDirectories: 'excluded-error',
      query: 'query-error', querySpec: batch ? 'batch-error' : 'advanced-error', extensions: 'extensions-error' }[error?.field] || 'general-error';
    view.set_text(target, message(error)); view.set_status('入力を確認してください。');
    view.set_text('empty-state', '検索を開始できませんでした。入力とエラーを確認してください。');
    Object.assign(state, { running: false, cancelling: false, currentId: null });
    view.disabled('search', false); view.disabled('cancel', true);
  }
}

async function onCancel() {
  if (!state.running || state.cancelling) return;
  state.cancelling = true;
  view.set_status('中断中…'); view.disabled('cancel', true);
  try { await invoke('cancel_search', { searchId: state.currentId }); } catch { /* Finish events own the search lifecycle. */ }
}

async function onClearIndex() {
  view.set_text('general-error', '');
  try {
    await invoke('clear_search_index', null);
    view.set_text('copy-status', '検索用索引を削除しました。次回、索引を有効にすると再作成します。');
  } catch (error) { view.set_text('general-error', message(error)); }
}

function onEvent(event) {
  if (!state.running || event.searchId !== state.currentId) return;
  const kind = event.type;
  if (kind === 'started') { view.report_started(event.searchId, event.request); view.set_status('列挙中…'); }
  if (kind === 'progress' || kind === 'finished') {
    const counts = event.counts || {}, n = key => counts[key] || 0;
    view.set_text('counts', kind === 'progress' && event.phase === 'discovery'
      ? `結果 ${n('resultCount')} · 発見 ${n('discoveredFiles')} · 処理 ${n('processedFiles')} · エラー ${n('issueCount')}`
      : `結果 ${n('resultCount')} · 処理 ${n('processedFiles')}/${n('discoveredFiles')} · エラー ${n('issueCount')}`);
    if (kind === 'progress' && !state.cancelling) view.set_status(event.phase === 'discovery' ? '列挙中…' : '検索中…');
  }
  if (kind === 'result') {
    const hit = structuredClone(event.hit);
    for (const item of hit.evidence || []) item.locationText = location(item.sourceKind, item.location);
    view.add_hit(hit, location(hit.sourceKind, hit.location));
  }
  if (kind === 'issue') view.add_issue(event.issue);
  if (kind === 'fileRanked') view.file_ranked(event.ranking);
  if (kind === 'rankingSummary') view.ranking_summary(event.files);
  if (kind === 'batchSummary') view.batch_summary(event.terms);
  if (kind === 'executionSummary') view.execution_summary(event);
  if (kind !== 'finished') return;
  view.rank_results();
  const reason = event.reason || 'failed', counts = event.counts || {};
  view.report_finished(reason, counts);
  view.set_status(reason === 'completed' ? counts.issueCount > 0 ? '完了（一部エラーあり）'
    : !counts.resultCount ? '完了 · 該当なし' : '完了' : reason === 'cancelled' ? '中断' : '全体失敗');
  if (!counts.resultCount) view.set_text('empty-state', reason === 'completed'
    ? '一致する結果はありませんでした。検索語や対象ファイルを確認してください。'
    : reason === 'cancelled' ? '検索は中断されました。条件を変えて再検索できます。' : '検索を完了できませんでした。上のエラーを確認してください。');
  Object.assign(state, { running: false, cancelling: false });
  view.disabled('search', false); view.disabled('cancel', true);
}

export function init() { return initialization ??= initialize(); }

async function initialize() {
  view.enable_more(); view.bind('clear-index', 'click', onClearIndex);
  view.init_result_filter(); view.init_report_actions(); view.init_extension_summary();
  view.init_search_mode(); view.init_edit_actions();
  // Register before enabling submission so immediate native events cannot be lost.
  view.disabled('search', true);
  await listen(onEvent);
  view.bind('search-form', 'submit', onSearch); view.bind('cancel', 'click', onCancel);
  await view.init_folder_lists();
  state.ready = true;
  view.disabled('search', false);
}
