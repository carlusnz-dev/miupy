// @req RF-25
import { api, type Dna } from '../ipc';
export class DnaStore {
  data = $state<Dna | null>(null);
  loading = $state(false);
  error = $state('');
  windowDays = $state(30);
  private generation = 0;
  async load(days = this.windowDays) {
    this.windowDays = days;
    const token = ++this.generation;
    this.loading = true; this.error = '';
    try { const data = await api.statsDna({ windowDays: days }); if (token === this.generation) this.data = data; }
    catch (e) { if (token === this.generation) { this.error = e instanceof Error ? e.message : 'Não foi possível carregar o DNA.'; this.data = null; } }
    finally { if (token === this.generation) this.loading = false; }
  }
}
export const dna = new DnaStore();
