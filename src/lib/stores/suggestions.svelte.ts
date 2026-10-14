// @req RF-23
import { api } from '../ipc';
export class SuggestionsStore {
  items = $state<string[]>([]);
  active = $state(-1);
  open = $state(false);
  private timer: ReturnType<typeof setTimeout> | undefined;
  private generation = 0;
  private settled = '';
  update(query: string) {
    if (query.trim() === this.settled) return;
    this.settled = '';
    clearTimeout(this.timer);
    const token = ++this.generation;
    this.items = []; this.active = -1; this.open = false;
    if (!query.trim()) return;
    this.timer = setTimeout(async () => {
      try { const items = await api.youtubeSuggest({ query: query.trim() }); if (token === this.generation) { this.items = items; this.open = items.length > 0; } }
      catch { if (token === this.generation) { this.items = []; this.open = false; } }
    }, 200);
  }
  move(delta: number) { if (this.items.length) { this.open = true; this.active = (this.active + delta + this.items.length) % this.items.length; } }
  /** Fecha a lista e cancela a consulta pendente; `settledQuery` não volta a abrir sugestões. */
  close(settledQuery?: string) { clearTimeout(this.timer); this.generation++; this.open = false; this.active = -1; if (settledQuery !== undefined) this.settled = settledQuery.trim(); }
  dispose() { clearTimeout(this.timer); this.generation++; }
}
