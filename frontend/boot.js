import('./app.js').then(({ init }) => init()).catch(error => {
  document.getElementById('search').disabled = true;
  document.getElementById('status').textContent = `画面を起動できません: ${error}`;
});
