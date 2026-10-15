// @req RF-29
// Estado do editor de barras do equalizador: perfil pronto abre só leitura, personalizado abre
// editável e "Criar perfil" parte da curva que está na tela.
import type { EqPreset } from '../ipc';

export interface EqDraft { id?: string; name: string; gains: number[]; readOnly: boolean }

export function eqDraft(preset: EqPreset | undefined, current: number[]): EqDraft {
  if (!preset) return { name: '', gains: [...current], readOnly: false };
  return { id: preset.id, name: preset.name, gains: [...preset.gainsDb], readOnly: preset.builtin };
}
