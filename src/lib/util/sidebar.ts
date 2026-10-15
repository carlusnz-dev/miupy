// @req RF-33
export const SIDEBAR_MIN = 220;
export const SIDEBAR_MAX = 400;
export const SIDEBAR_DEFAULT = 280;
export const SIDEBAR_STEP = 8;
export const SIDEBAR_KEY = 'miupy.sidebarWidth';
export const SIDEBAR_COLLAPSED_KEY = 'miupy.sidebarCollapsed';

export function clampSidebarWidth(width: number): number {
  if (!Number.isFinite(width)) return SIDEBAR_DEFAULT;
  return Math.max(SIDEBAR_MIN, Math.min(SIDEBAR_MAX, Math.round(width)));
}

export function draggedSidebarWidth(startWidth: number, startX: number, currentX: number): number {
  return clampSidebarWidth(startWidth + currentX - startX);
}

export function keyboardSidebarWidth(width: number, key: string): number | null {
  if (key === 'ArrowRight') return clampSidebarWidth(width + SIDEBAR_STEP);
  if (key === 'ArrowLeft') return clampSidebarWidth(width - SIDEBAR_STEP);
  return null;
}

// @req RF-33: pré-visualiza no movimento; persiste somente no término.
export function beginSidebarDrag(startWidth: number, startX: number, preview: (width: number) => void, save: (width: number) => void) {
  let width = clampSidebarWidth(startWidth);
  return {
    move(currentX: number) {
      width = draggedSidebarWidth(startWidth, startX, currentX);
      preview(width);
    },
    finish() {
      if (width !== startWidth) save(width);
    },
    cancel() {
      preview(startWidth);
    },
  };
}

export const SIDEBAR_KEY_SAVE_MS = 400;

// @req RF-33: no teclado cada tecla só pré-visualiza; salva o valor final depois de uma pausa ou ao sair do foco.
export function createDeferredSave(save: (width: number) => void, delayMs = SIDEBAR_KEY_SAVE_MS) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pending: number | null = null;
  const clear = () => { if (timer) clearTimeout(timer); timer = null; };
  const flush = () => {
    clear();
    if (pending === null) return;
    const width = pending;
    pending = null;
    save(width);
  };
  return {
    schedule(width: number) {
      pending = width;
      clear();
      timer = setTimeout(flush, delayMs);
    },
    flush,
    /** Descarta o que estava esperando (outro caminho, como o arraste, vai salvar). */
    cancel() { clear(); pending = null; },
  };
}

// @req RF-38: o estado recolhido fica só no cache local; qualquer valor diferente de "1" é expandido.
export function parseCollapsed(raw: string | null | undefined): boolean {
  return raw === '1';
}
