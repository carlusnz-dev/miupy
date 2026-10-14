// @req RF-19 RNF-11 RNF-13
import { api, type Recommendation } from '../ipc';
import { reconcileList } from '../util/diff';

/** Última lista de cada contexto (discoteca ou playlist), para a próxima tela mostrar na hora. */
const memory = new Map<string, Recommendation[]>();
export function clearRecommendationsCache() { memory.clear(); }

export class RecommendationsStore {
  items = $state.raw<Recommendation[]>([]);
  loading = $state(false);
  /** Já há uma resposta (da rede ou do cache): antes disso a tela mostra o esqueleto. */
  ready = $state(false);
  refreshing = $state(false);
  error = $state<string | null>(null);
  private unsubscribe: (() => void) | null = null;
  constructor(readonly playlistId?: string) {
    const cached = memory.get(playlistId ?? '');
    if (cached) { this.items = cached; this.ready = true; }
  }
  private set(items: Recommendation[]) {
    const merged = reconcileList(this.ready ? this.items : undefined, items, (r) => r.sourceId);
    if (merged !== this.items) this.items = merged;
    memory.set(this.playlistId ?? '', merged);
    this.ready = true;
  }

  async start() {
    if (!this.unsubscribe) this.unsubscribe = await api.on('recommendations://changed', () => { void this.load(); });
    await this.load();
  }
  stop() { this.unsubscribe?.(); this.unsubscribe = null; }
  async load() {
    this.loading = true;
    try { this.set(await api.recommendationsList({ playlistId: this.playlistId })); this.error = null; }
    catch (e) { this.error = (e as { message?: string })?.message ?? 'Falha ao carregar sugestões.'; }
    finally { this.loading = false; }
  }
  async refresh() {
    this.refreshing = true;
    try { this.set(await api.recommendationsRefresh({ playlistId: this.playlistId })); this.error = null; }
    catch (e) { this.error = (e as { code?: string })?.code === 'network' ? 'Sem conexão. Suas sugestões anteriores continuam disponíveis.' : (e as { message?: string })?.message ?? 'Falha ao atualizar sugestões.'; }
    finally { this.refreshing = false; }
  }
  async dismiss(sourceId: string) {
    await api.recommendationsDismiss({ sourceId });
    this.set(this.items.filter((r) => r.sourceId !== sourceId));
  }
  async add(sourceId: string) {
    await api.youtubeAddToLibrary({ sourceIds: [sourceId], playlistId: this.playlistId });
    this.set(this.items.filter((r) => r.sourceId !== sourceId));
  }
}
