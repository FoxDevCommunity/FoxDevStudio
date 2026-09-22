// Loads the FoxVM wasm module from the inlined base64 payload. One code path for Vite dev,
// the packaged app (file://, inside asar) and vitest/jsdom.
import * as foxvm from './generated/foxvm.js';
import { FOXVM_WASM_B64 } from './generated/foxvm_wasm';

export type FoxVmModule = typeof foxvm;

let loaded: FoxVmModule | null = null;
let pending: Promise<FoxVmModule> | null = null;

function decodeBase64(b64: string): Uint8Array {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

/** Synchronous load; instantiation is a few milliseconds and happens once per process. */
export function loadFoxVmSync(): FoxVmModule {
  if (!loaded) {
    foxvm.initSync({ module: decodeBase64(FOXVM_WASM_B64) });
    loaded = foxvm;
  }
  return loaded;
}

export function loadFoxVm(): Promise<FoxVmModule> {
  pending ??= Promise.resolve().then(loadFoxVmSync);
  return pending;
}
