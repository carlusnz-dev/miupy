// @req RF-19
import { api, type Recommendation } from '../ipc';

export class RecommendationsStore {
  items = $state<Recommendation[]>([]);
  loading = $state(false);
  refreshing = $state(false);
  error = $state<string | null>(null);
  private unsubscribe: (() => void) | null = null;
  constructor(readonly playlistId?: string) {}

  async start() {
    if (!this.unsubscribe) this.unsubscribe = await api.on('recommendations://changed', () => { void this.load(); });
    await this.load();
  }
  stop() { this.unsubscribe?.(); this.unsubscribe = null; }
  async load() {
    this.loading = true;
    try { this.items = await api.recommendationsList({ playlistId: this.playlistId }); this.error = null; }
    catch (e) { this.error = (e as { message?: string })?.message ?? 'Falha ao carregar sugestões.'; }
    finally { this.loading = false; }
  }
  async refresh() {
    this.refreshing = true;
    try { this.items = await api.recommendationsRefresh({ playlistId: this.playlistId }); this.error = null; }
    catch (e) { this.error = (e as { code?: string })?.code === 'network' ? 'Sem conexão. Suas sugestões anteriores continuam disponíveis.' : (e as { message?: string })?.message ?? 'Falha ao atualizar sugestões.'; }
    finally { this.refreshing = false; }
  }
  async dismiss(sourceId: string) {
    await api.recommendationsDismiss({ sourceId });
    this.items = this.items.filter((r) => r.sourceId !== sourceId);
  }
  async add(sourceId: string) {
    await api.youtubeAddToLibrary({ sourceIds: [sourceId], playlistId: this.playlistId });
    this.items = this.items.filter((r) => r.sourceId !== sourceId);
  }
}
