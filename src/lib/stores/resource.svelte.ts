// @req RNF-11 RNF-13
import { untrack } from 'svelte';
import { mergeDeep } from '../util/diff';
import type { CacheTag } from '../util/libraryEvents';

/**
 * Cache de respostas em memória durante a sessão, por chave de consulta (RNF-11). Uma tela que volta
 * a uma consulta já feita mostra o cache na hora e atualiza em segundo plano; sem cache, mostra o
 * esqueleto (`Resource.loading`).
 */
const memory = new Map<string, unknown>();
const live = new Set<Resource<any>>(); // eslint-disable-line @typescript-eslint/no-explicit-any

/** Espera mínima para juntar eventos em sequência numa atualização só. */
export const SETTLE_MS = 120;
/** Intervalo mínimo entre duas atualizações do mesmo recurso, mesmo com uma rajada de eventos. */
export const MIN_INTERVAL_MS = 500;

/** Esquece o cache (testes e "sair da conta"). */
export function clearCache() { memory.clear(); }
export function cachedValue<T>(key: string): T | undefined { return memory.get(key) as T | undefined; }

/** Um evento mudou os dados desses grupos: os recursos abertos que dependem deles se atualizam. */
export function invalidate(tags: readonly CacheTag[]) {
  for (const r of live) if (r.tags.some((t) => tags.includes(t))) r.schedule();
}

function text(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e) return String((e as { message: unknown }).message);
  return typeof e === 'string' ? e : 'Algo deu errado.';
}

export class Resource<T> {
  /** Última resposta (do cache ou da rede); `undefined` até existir uma. */
  data = $state.raw<T | undefined>(undefined);
  error = $state<string | null>(null);
  /** Chave da consulta a que `data` pertence (telas com várias chaves, como o ano da retrospectiva, a comparam com a atual). */
  dataKey = $state('');
  /** Há uma consulta em andamento (inclusive em segundo plano, com `data` já na tela). */
  refreshing = $state(false);

  private key = '';
  private fetcher: (() => Promise<T>) | null = null;
  private gen = 0;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private lastRun = 0;
  private dirty = false;

  /**
   * @param tags grupos de `library://changed` que atualizam este recurso enquanto a tela está aberta.
   * @param merge decide o que fica: devolver `prev` mantém as referências e evita repintar.
   */
  constructor(readonly tags: readonly CacheTag[], private merge: (prev: T | undefined, next: T) => T = mergeDeep) {}

  /** Sem dado nenhum e sem erro: a tela mostra o esqueleto. */
  get loading() { return this.data === undefined && this.error === null; }

  /** Mostra na hora o cache de `key`, se houver. */
  prime(key: string) {
    untrack(() => {
      this.key = key;
      if (!memory.has(key)) return;
      const merged = this.merge(this.data, memory.get(key) as T);
      if (merged !== this.data) this.data = merged;
      this.dataKey = key;
    });
  }

  /** Mostra o cache de `key` e consulta de novo em segundo plano. */
  load(key: string, fetcher: () => Promise<T>) {
    this.fetcher = fetcher;
    this.prime(key);
    return this.run();
  }

  /** Repete a última consulta (botão "Tentar de novo"). */
  reload() { return this.run(); }

  /** Registra o recurso para receber `invalidate`; devolve a função que desfaz. Use num `$effect`. */
  attach() {
    live.add(this);
    return () => {
      live.delete(this);
      if (this.timer) clearTimeout(this.timer);
      this.timer = null;
    };
  }

  /** Agenda uma atualização juntando rajadas: no máximo uma a cada `MIN_INTERVAL_MS`. */
  schedule() {
    if (!this.fetcher) return;
    if (this.refreshing) { this.dirty = true; return; }
    if (this.timer) return;
    const wait = Math.max(SETTLE_MS, this.lastRun + MIN_INTERVAL_MS - Date.now());
    this.timer = setTimeout(() => { this.timer = null; void this.run(); }, wait);
  }

  private async run() {
    const fetcher = this.fetcher;
    if (!fetcher) return;
    const token = ++this.gen;
    const key = this.key;
    this.lastRun = Date.now();
    untrack(() => { this.refreshing = true; });
    try {
      const next = await fetcher();
      if (token !== this.gen) return;
      const merged = untrack(() => this.merge(this.data, next));
      memory.set(key, merged);
      if (merged !== this.data) this.data = merged;
      if (this.key === key) this.dataKey = key;
      if (this.error !== null) this.error = null;
    } catch (e) {
      if (token === this.gen) this.error = text(e);
    } finally {
      if (token === this.gen) {
        this.refreshing = false;
        if (this.dirty) { this.dirty = false; this.schedule(); }
      }
    }
  }
}
