// The transport is supplied by Tauri in the EXE and replaced only at the boundary in UI tests.
export function invoke(command, args) {
  return window.__TAURI__.core.invoke(command, args);
}

export function listen(callback) {
  return window.__TAURI__.event.listen('search-events', event => {
    for (const item of event.payload) callback(item);
  });
}
