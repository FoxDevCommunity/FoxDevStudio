import type { FoxDevApi } from '@shared/ipc/api';

declare global {
  interface Window {
    foxdev?: FoxDevApi;
  }
}
export {};
