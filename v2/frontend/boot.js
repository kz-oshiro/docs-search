import init from './pkg/doc_search_ui.js';
init().catch(error => { document.getElementById('status').textContent = `画面を起動できません: ${error}`; });
