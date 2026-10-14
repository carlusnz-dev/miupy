// @req RF-24
import type { ShuffleMode } from '../ipc/types';
export function nextShuffle(mode: ShuffleMode): ShuffleMode { return mode === 'off' ? 'random' : mode === 'random' ? 'smart' : 'off'; }
export function shuffleLabel(mode: ShuffleMode): string { return mode === 'off' ? 'Aleatório desligado' : mode === 'random' ? 'Aleatório ligado' : 'Aleatório inteligente'; }
