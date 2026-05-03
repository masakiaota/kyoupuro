export function readSessionState(key) {
  try {
    const raw = window.sessionStorage.getItem(key);
    if (!raw) {
      return {};
    }
    const value = JSON.parse(raw);
    return value && typeof value === "object" ? value : {};
  } catch {
    return {};
  }
}

export function writeSessionState(key, value) {
  try {
    window.sessionStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage can be unavailable in strict browser privacy modes.
  }
}
