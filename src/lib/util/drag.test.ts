// @test RF-46
import { describe, expect, it } from 'vitest';
import { beginDrag } from './drag';

describe('gestos do player no celular', () => {
  it('aceita distância no sentido certo e recusa abaixo do limiar', () => {
    const drag = beginDrag(0, 0);
    drag.move(34, 100);
    expect(drag.finish(100, 1, 0.35)).toBe(false);
    drag.move(35, 200);
    expect(drag.finish(100, 1, 0.35)).toBe(true);
    expect(drag.finish(100, -1, 0.35)).toBe(false);
  });
  it('aceita gesto rápido e rejeita cancelamento', () => {
    const drag = beginDrag(0, 0);
    drag.move(30, 30);
    expect(drag.finish(200, 1, 0.35)).toBe(true);
    expect(drag.finish(200, 1, 0.35, 200)).toBe(false);
    drag.cancel();
    expect(drag.finish(200, 1, 0.35)).toBe(false);
  });
  it('não conta como gesto um toque que escorrega poucos pixels', () => {
    const drag = beginDrag(0, 0);
    drag.move(10, 10);
    expect(drag.finish(100, 1, 0.35)).toBe(false);
  });
  it('mantém o limiar por fração em telas pequenas', () => {
    const drag = beginDrag(0, 0);
    drag.move(35, 500);
    expect(drag.finish(100, 1, 0.35)).toBe(true);
  });
  it('reconhece o deslizar para a esquerda', () => {
    const drag = beginDrag(100, 0);
    drag.move(64, 200);
    expect(drag.finish(100, -1, 0.35)).toBe(true);
    expect(drag.finish(100, 1, 0.35)).toBe(false);
  });
  // @test RF-47
  it('abre e recolhe a folha com o mesmo limiar nos dois sentidos', () => {
    const opening = beginDrag(300, 0);
    opening.move(220, 200);
    expect(opening.finish(300, -1, 0.3)).toBe(false);
    opening.move(210, 300);
    expect(opening.finish(300, -1, 0.3)).toBe(true);

    const closing = beginDrag(100, 0);
    closing.move(130, 30);
    expect(closing.finish(300, 1, 0.3)).toBe(true);
  });
});
