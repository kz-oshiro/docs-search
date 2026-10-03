import init from './pkg/docs_search_ui.js';
init().catch(error => { document.getElementById('status').textContent = `画面を起動できません: ${error}`; });
