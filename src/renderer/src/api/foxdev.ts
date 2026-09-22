import type { FoxDevApi } from '@shared/ipc/api';
import { createMemoryApi } from './memoryApi';

let cached: FoxDevApi | undefined;

/** Returns the preload-exposed API, or an in-memory fallback outside Electron. */
export function getApi(): FoxDevApi {
  if (!cached) cached = typeof window !== 'undefined' && window.foxdev ? window.foxdev : createMemoryApi();
  return cached;
}

/** Test hook: replace the API instance. */
export function setApi(api: FoxDevApi | undefined): void {
  cached = api;
}
