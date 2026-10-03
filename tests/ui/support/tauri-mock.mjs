// Installed before theme.js, boot.js and the real frontend entry point.
// Only the Tauri/OS boundary is replaced. No rendering or search logic lives here.
export function installTauriMock(options = {}) {
  const calls = [], events = [], faults = [], clipboard = [];
  const listeners = new Map(), queues = new Map(), deferred = new Map(), sequences = new Map();
  if (options.storage && !sessionStorage.getItem('docs-search.ui-test-seeded')) {
    for (const [key, value] of Object.entries(options.storage)) localStorage.setItem(key, value);
    sessionStorage.setItem('docs-search.ui-test-seeded', '1');
  }
  if (options.storageReadError) {
    const original = Storage.prototype.getItem;
    Storage.prototype.getItem = function(key) {
      if (this === localStorage) throw new DOMException('Test storage read denied', 'SecurityError');
      return original.call(this, key);
    };
  }
  if (options.storageWriteError) {
    const original = Storage.prototype.setItem;
    Storage.prototype.setItem = function(key, value) {
      if (this === localStorage) throw new DOMException('Test storage write denied', 'QuotaExceededError');
      return original.call(this, key, value);
    };
  }
  for (const [command, responses] of Object.entries(options.responses ?? {})) queues.set(command, [...responses]);
  function settle(response) {
    if (response?.error !== undefined) return Promise.reject(response.error);
    return Promise.resolve(response?.value ?? null);
  }
  async function invoke(command, args) {
    calls.push({ command, args: structuredClone(args ?? null) });
    const queue = queues.get(command);
    if (queue?.length) {
      const response = queue.shift();
      if (response.deferred) {
        if (deferred.has(response.deferred)) throw new Error('Duplicate deferred key');
        return new Promise((resolve, reject) => deferred.set(response.deferred, { resolve, reject }));
      }
      return settle(response);
    }
    if (['start_search', 'cancel_search', 'clear_search_index', 'open_result', 'cancel_result_edit'].includes(command)) return null;
    if (command === 'validate_folder_paths') return args.paths;
    if (command === 'preview_batch') return []; // Tests needing a count supply a backend response.
    const message = `Unconfigured Tauri command: ${command}`;
    faults.push(message);
    throw new Error(message);
  }
  const control = {
    calls, events, faults, clipboard, clipboardError: false,
    ready: false,
    queue(command, response) {
      if (!queues.has(command)) queues.set(command, []);
      queues.get(command).push(response);
    },
    resolve(key, response) {
      const pending = deferred.get(key);
      if (!pending) throw new Error(`No pending response: ${key}`);
      deferred.delete(key);
      if (response?.error !== undefined) pending.reject(response.error);
      else pending.resolve(response?.value ?? null);
    },
    pending() { return [...deferred.keys()]; },
    emit(items, searchId) {
      const payload = items.map(item => {
        const id = item.searchId ?? searchId;
        if (!id) throw new Error('An explicit or captured searchId is required.');
        const sequence = (sequences.get(id) ?? 0) + 1;
        sequences.set(id, sequence);
        return { searchId: id, sequence, ...item };
      });
      events.push(...structuredClone(payload));
      const callback = listeners.get('search-events');
      if (!callback) throw new Error('Frontend event listener is not ready.');
      callback({ event: 'search-events', payload });
    },
  };
  window.__docsSearchTest = control;
  window.__TAURI__ = {
    core: { invoke },
    event: {
      async listen(name, callback) {
        if (name !== 'search-events' || listeners.has(name)) {
          const message = `Unexpected event subscription: ${name}`;
          faults.push(message);
          throw new Error(message);
        }
        listeners.set(name, callback);
        control.ready = true;
        return () => listeners.delete(name);
      },
    },
  };
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: {
      async writeText(value) {
        if (control.clipboardError) throw new Error('Test clipboard denied');
        clipboard.push(value);
      },
    },
  });
}
