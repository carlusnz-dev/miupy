import type { MiupyApi } from './api';
import { createTauriApi } from './tauri';
import { createMockApi } from './mock';

export * from './types';
export type { MiupyApi } from './api';
export { toIpcError } from './tauri';

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Cliente IPC: real dentro do Tauri, mock em memória no navegador puro. */
export const api: MiupyApi = inTauri ? createTauriApi() : createMockApi();
export const isMock = !inTauri;

export function errorMessage(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e) return String((e as { message: unknown }).message);
  return typeof e === 'string' ? e : 'Algo deu errado.';
}
