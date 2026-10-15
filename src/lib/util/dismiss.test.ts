import { describe, expect, it, vi } from 'vitest';
import { dismissOnEscapeOrOutside } from './dismiss';

const key = (name: string) => Object.assign(new Event('keydown'), { key: name });

describe('fechar popup', () => {
  // @test RF-29
  it('Escape e clique fora fecham; clique dentro e outras teclas não', () => {
    const root = new EventTarget();
    const inside = {};
    const outside = {};
    const close = vi.fn();
    dismissOnEscapeOrOutside(root, (t) => t === inside, close);

    root.dispatchEvent(key('Enter'));
    expect(close).not.toHaveBeenCalled();

    const click = (target: object) => {
      const event = new Event('pointerdown');
      Object.defineProperty(event, 'target', { value: target });
      root.dispatchEvent(event);
    };
    click(inside);
    expect(close).not.toHaveBeenCalled();

    click(outside);
    expect(close).toHaveBeenLastCalledWith('outside');
    root.dispatchEvent(key('Escape'));
    expect(close).toHaveBeenLastCalledWith('escape');
    expect(close).toHaveBeenCalledTimes(2);
  });

  // @test RF-29
  it('depois de desfeito, não escuta mais', () => {
    const root = new EventTarget();
    const close = vi.fn();
    const off = dismissOnEscapeOrOutside(root, () => false, close);
    off();
    root.dispatchEvent(key('Escape'));
    expect(close).not.toHaveBeenCalled();
  });
});
