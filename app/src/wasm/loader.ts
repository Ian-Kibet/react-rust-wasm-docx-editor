import init from './pkg/docx_wasm.js';

let initialized = false;
let initPromise: Promise<void> | null = null;

export async function initWasm(): Promise<void> {
  if (initialized) return;
  if (initPromise) return initPromise;

  initPromise = init().then(() => {
    initialized = true;
  });

  return initPromise;
}

export function isWasmReady(): boolean {
  return initialized;
}
