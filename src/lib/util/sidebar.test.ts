import { describe, expect, it, vi } from 'vitest';
import { beginSidebarDrag, clampSidebarWidth, draggedSidebarWidth, keyboardSidebarWidth } from './sidebar';

describe('largura do menu lateral', () => {
  // @test RF-33
  it('corta a largura, inclusive após arraste e teclas', () => {
    expect(clampSidebarWidth(10)).toBe(220);
    expect(clampSidebarWidth(999)).toBe(400);
    expect(clampSidebarWidth(Number.NaN)).toBe(280);
    expect(draggedSidebarWidth(280, 100, 145)).toBe(325);
    expect(draggedSidebarWidth(280, 100, -100)).toBe(220);
    expect(keyboardSidebarWidth(280, 'ArrowRight')).toBe(288);
    expect(keyboardSidebarWidth(220, 'ArrowLeft')).toBe(220);
    expect(keyboardSidebarWidth(280, 'Enter')).toBeNull();
  });

  // @test RF-33
  it('pré-visualiza sem salvar e persiste uma vez ao soltar', () => {
    const preview = vi.fn();
    const save = vi.fn();
    const drag = beginSidebarDrag(280, 100, preview, save);
    drag.move(130);
    drag.move(900);
    expect(preview.mock.calls).toEqual([[310], [400]]);
    expect(save).not.toHaveBeenCalled();
    drag.finish();
    expect(save).toHaveBeenCalledOnce();
    expect(save).toHaveBeenCalledWith(400);
    const cancelled = beginSidebarDrag(280, 100, preview, save);
    cancelled.move(150);
    cancelled.cancel();
    expect(preview).toHaveBeenLastCalledWith(280);
    expect(save).toHaveBeenCalledTimes(1);
  });
});
