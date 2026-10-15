// @req RF-33
export const SIDEBAR_MIN = 220;
export const SIDEBAR_MAX = 400;
export const SIDEBAR_DEFAULT = 280;
export const SIDEBAR_STEP = 8;
export const SIDEBAR_KEY = 'miupy.sidebarWidth';

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
