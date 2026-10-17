// @req RF-40 RNF-04
// Cor dominante da capa da faixa atual: o Tocando a usa no mesh e `--cover-tint` (no :root) tinge o vidro e o
// brilho ambiente. Sem capa, ou com canvas "tainted", `colors` fica `undefined` e a variável sai: vale a lava.
import { api } from '../ipc';
import { assetCrossOrigin } from '../util/asset';
import { coverPalette, meshTriple } from '../util/palette';
import { player } from './player.svelte';

class CoverStore {
  /** Endereço da capa da faixa atual (arquivo local ou miniatura remota). */
  readonly src = $derived(api.resolveAsset(player.track?.coverPath ?? null) ?? player.track?.thumbnailUrl ?? undefined);
  colors = $state<[string, string, string] | undefined>();
  private started = false;

  start() {
    if (this.started) return;
    this.started = true;
    $effect.root(() => {
      $effect(() => {
        const src = this.src;
        if (!src) { this.colors = undefined; return; }
        let cancelled = false;
        void coverPalette(src, assetCrossOrigin(src)).then((palette) => { if (!cancelled) this.colors = meshTriple(palette); });
        return () => { cancelled = true; };
      });
      $effect(() => {
        const root = document.documentElement.style;
        if (this.colors) root.setProperty('--cover-tint', this.colors[0]); else root.removeProperty('--cover-tint');
      });
    });
  }
}
export const cover = new CoverStore();
