// Appearance preferences stay in the WebView; search requests never include them.
(() => {
  const storageKey = 'docs-search.theme';
  const defaultTheme = 'sun';
  const themes = new Set(['sun', 'amber', 'sunset', 'teal', 'blue', 'forest']);
  const normalizeTheme = value => themes.has(value) ? value : defaultTheme;
  let canSave = true;
  let theme = defaultTheme;
  try {
    theme = normalizeTheme(window.localStorage.getItem(storageKey));
  } catch {
    canSave = false;
  }
  // This script runs in the head, before the search screen is initialized.
  document.documentElement.dataset.theme = theme;

  function initSettings() {
    const settings = document.getElementById('appearance-settings');
    const select = document.getElementById('theme-color');
    const message = document.getElementById('theme-save-status');
    select.value = theme;
    function updateMessage() {
      message.textContent = canSave
        ? '選んだ色は次回起動時にも使います。'
        : 'テーマを保存できないため、この起動中だけ反映します。';
    }
    updateMessage();
    select.addEventListener('change', () => {
      theme = normalizeTheme(select.value);
      select.value = theme;
      document.documentElement.dataset.theme = theme;
      try {
        window.localStorage.setItem(storageKey, theme);
        canSave = true;
      } catch {
        canSave = false;
      }
      updateMessage();
    });
    settings.addEventListener('keydown', event => {
      if (event.key === 'Escape') {
        settings.open = false;
        settings.querySelector('summary').focus();
      }
    });
    document.addEventListener('click', event => {
      if (settings.open && !settings.contains(event.target)) settings.open = false;
    });
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initSettings, { once: true });
  } else {
    initSettings();
  }
})();
