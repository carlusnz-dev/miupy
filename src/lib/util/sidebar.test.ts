import { describe, expect, it, vi } from 'vitest';
import { beginSidebarDrag, clampSidebarWidth, createDeferredSave, draggedSidebarWidth, keyboardSidebarWidth } from './sidebar';

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

describe('largura pelo teclado', () => {
  // @test RF-33
  it('várias teclas seguidas gravam só o valor final, depois de uma pausa', () => {
    vi.useFakeTimers();
    const save = vi.fn();
    const deferred = createDeferredSave(save, 400);
    let width = 280;
    for (let i = 0; i < 5; i++) {
      width = keyboardSidebarWidth(width, 'ArrowRight')!;
      deferred.schedule(width);
      vi.advanceTimersByTime(100);
    }
    expect(save).not.toHaveBeenCalled();
    vi.advanceTimersByTime(400);
    expect(save.mock.calls).toEqual([[320]]);
    vi.useRealTimers();
  });

  // @test RF-33
  it('perder o foco grava na hora e só uma vez; cancelar descarta', () => {
    vi.useFakeTimers();
    const save = vi.fn();
    const deferred = createDeferredSave(save, 400);
    deferred.schedule(300);
    deferred.flush();
    expect(save.mock.calls).toEqual([[300]]);
    vi.advanceTimersByTime(1000);
    deferred.flush();
    expect(save).toHaveBeenCalledTimes(1);

    deferred.schedule(310);
    deferred.cancel();
    vi.advanceTimersByTime(1000);
    deferred.flush();
    expect(save).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });
});
