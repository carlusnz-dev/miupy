import { describe, expect, it } from 'vitest';
import type { EqPreset } from '../ipc';
import { eqDraft } from './eqEditor';

const rock: EqPreset = { id: 'rock', name: 'Rock', builtin: true, gainsDb: [4, 3, 1, 0, 0, 1, 3, 4] };
const mine: EqPreset = { id: 'u1', name: 'Meu', builtin: false, gainsDb: [1, 0, 0, 0, 0, 0, 0, -1] };

describe('editor do equalizador', () => {
  // @test RF-29
  it('perfil pronto mostra a curva dele e abre só leitura', () => {
    expect(eqDraft(rock, [0, 0, 0, 0, 0, 0, 0, 0])).toEqual({ id: 'rock', name: 'Rock', gains: rock.gainsDb, readOnly: true });
  });

  // @test RF-29
  it('perfil personalizado abre editável, em cópia', () => {
    const draft = eqDraft(mine, rock.gainsDb);
    expect(draft.readOnly).toBe(false);
    expect(draft.gains).toEqual(mine.gainsDb);
    expect(draft.gains).not.toBe(mine.gainsDb);
  });

  // @test RF-29
  it('criar perfil parte da curva atual, sem id e sem nome', () => {
    const current = [2, 2, 0, 0, 0, 0, 1, 1];
    const draft = eqDraft(undefined, current);
    expect(draft).toEqual({ name: '', gains: current, readOnly: false });
    expect(draft.gains).not.toBe(current);
  });
});
