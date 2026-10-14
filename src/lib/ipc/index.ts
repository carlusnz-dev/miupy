import type { MiupyApi } from './api';
import { createTauriApi } from './tauri';

export * from './types';
export type { MiupyApi } from './api';
export { toIpcError } from './tauri';

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Cliente IPC: real dentro do Tauri, mock em memória no navegador puro. */
// O mock só é importado em desenvolvimento: fica fora do bundle de produção.
export const api: MiupyApi = inTauri ? createTauriApi() : import.meta.env.DEV
  ? (await import('./mock')).createMockApi()
  : createTauriApi();
export const isMock = !inTauri && import.meta.env.DEV;

export function errorMessage(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e) return String((e as { message: unknown }).message);
  return typeof e === 'string' ? e : 'Algo deu errado.';
}
