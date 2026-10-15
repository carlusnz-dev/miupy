// @req RF-34 RNF-11
import { api } from '../ipc';

/** Buscas recentes do YouTube, mostradas com a caixa de busca vazia. */
export class SearchHistoryStore {
  items = $state<string[]>([]);
  /** Já há uma resposta: antes disso a tela mostra o esqueleto. */
  ready = $state(false);
  error = $state<string | null>(null);
  private generation = 0;

  async load() {
    const token = ++this.generation;
    try {
      const items = await api.youtubeSearchHistory();
      if (token === this.generation) { this.items = items; this.error = null; }
    } catch (e) {
      if (token === this.generation) this.error = (e as { message?: string })?.message ?? 'Falha ao carregar as buscas recentes.';
    } finally {
      if (token === this.generation) this.ready = true;
    }
  }

  async clear() {
    const token = ++this.generation;
    await api.youtubeSearchHistoryClear();
    if (token === this.generation) { this.items = []; this.error = null; this.ready = true; }
  }
}
