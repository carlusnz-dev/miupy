// @req RF-09 RF-34
import { api, errorMessage, type SearchResult } from '../ipc';

/** Estado da busca no YouTube; respostas de buscas antigas ou canceladas são descartadas. */
export class YoutubeSearchStore {
  results = $state<SearchResult[] | null>(null);
  searching = $state(false);
  error = $state<string | null>(null);
  private seq = 0;

  async run(query: string) {
    const q = query.trim();
    if (!q) return;
    const token = ++this.seq;
    this.searching = true; this.error = null;
    try {
      const found = await api.youtubeSearch({ query: q, limit: 20 });
      if (token === this.seq) this.results = found;
    } catch (ex) {
      if (token === this.seq) { this.error = errorMessage(ex); this.results = null; }
    } finally {
      if (token === this.seq) this.searching = false;
    }
  }

  /** Caixa esvaziada: cancela a busca em andamento e volta ao estado inicial. */
  reset() {
    this.seq++;
    this.searching = false; this.error = null; this.results = null;
  }
}
