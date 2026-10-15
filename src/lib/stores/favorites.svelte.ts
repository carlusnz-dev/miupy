// @req RF-04
import { SvelteMap } from 'svelte/reactivity';
import type { Track } from '../ipc';

/**
 * Quanto o valor otimista segue valendo depois de o comando confirmar: tempo para o
 * `library://changed` chegar e a lista atualizar (MIN_INTERVAL_MS do `Resource` mais a consulta).
 */
export const CONFIRM_HOLD_MS = 1500;

/**
 * Coração atualizado na hora: guarda por faixa o valor que o usuário acabou de pedir e o mostra no
 * lugar de `track.isFavorite` até o comando falhar (volta ao estado anterior) ou a lista nova chegar.
 */
class FavoriteOverlay {
  private readonly pending = new SvelteMap<string, boolean>();
  private readonly tokens = new Map<string, number>();
  private n = 0;

  /** Valor a mostrar para a faixa: o otimista, se houver; senão o que veio do backend. */
  is(track: Pick<Track, 'id' | 'isFavorite'>): boolean {
    return this.pending.get(track.id) ?? track.isFavorite;
  }

  /** Aplica `favorite` na hora; devolve o que confirma ou desfaz depois que o comando responder. */
  apply(trackId: string, favorite: boolean) {
    const token = ++this.n;
    this.tokens.set(trackId, token);
    this.pending.set(trackId, favorite);
    const release = () => {
      if (this.tokens.get(trackId) !== token) return; // um toque mais novo assumiu
      this.tokens.delete(trackId);
      this.pending.delete(trackId);
    };
    return {
      confirm: () => { setTimeout(release, CONFIRM_HOLD_MS); },
      revert: release,
    };
  }
}
export const favorites = new FavoriteOverlay();
