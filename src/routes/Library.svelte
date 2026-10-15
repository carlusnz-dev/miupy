<script lang="ts">
  // Discoteca: busca, ordenação, filtro de favoritas e seleção múltipla.
  // @req RF-05 RF-08 RF-10 RNF-11 RNF-13
  import { untrack } from 'svelte';
  import { api, type SortDir, type Track, type TrackSort } from '../lib/ipc';
  import { Resource } from '../lib/stores/resource.svelte';
  import { mergeList } from '../lib/util/diff';
  import { Chunked } from '../lib/util/chunked.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { Selection } from '../lib/stores/selection.svelte';
  import { ui } from '../lib/stores/ui.svelte';
  import { href, paths } from '../lib/stores/router.svelte';
  import { downloadTracks, removeTracks } from '../lib/actions';
  import { fmtDuration } from '../lib/util/format';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import TrackToolbar from '../lib/components/TrackToolbar.svelte';
  import TrackRow from '../lib/components/TrackRow.svelte';
  import State from '../lib/components/State.svelte';
  import SkeletonList from '../lib/components/SkeletonList.svelte';
  import Icon from '../lib/components/Icon.svelte';

  let search = $state('');
  let sortBy = $state<TrackSort>('addedAt');
  let sortDir = $state<SortDir>('desc');
  let favoritesOnly = $state(false);
  // Colunas extras (adicionada há, gêneros): a escolha fica salva neste dispositivo.
  const INFO_KEY = 'miupy.libraryInfo';
  let info = $state((() => { try { return localStorage.getItem(INFO_KEY) === '1'; } catch { return false; } })());
  function toggleInfo() {
    info = !info;
    try { localStorage.setItem(INFO_KEY, info ? '1' : '0'); } catch { /* sem storage: ok */ }
  }
  const sel = new Selection();

  // Cache por consulta: voltar à Discoteca mostra a lista na hora e atualiza em segundo plano; um
  // library://changed só troca as linhas que mudaram (RNF-11, RNF-13).
  const res = new Resource<Track[]>(['tracks'], mergeList((t: Track) => t.id));
  const tracks = $derived(res.data ?? []);
  const loading = $derived(res.loading);
  const error = $derived(res.data === undefined ? res.error : null);
  $effect(() => res.attach());

  // carrega quando muda a consulta (com debounce na busca); o cache da consulta aparece sem esperar
  $effect(() => {
    sortBy; sortDir; favoritesOnly;
    const q = search.trim();
    const query = { search: q || undefined, sortBy, sortDir, favoritesOnly };
    const run = () => { void res.load(`tracks:${JSON.stringify(query)}`, () => api.libraryListTracks({ query })); };
    if (!q) { run(); return; }
    const t = setTimeout(run, 200);
    return () => clearTimeout(t);
  });
  // tira da seleção o que sumiu da lista
  $effect(() => {
    const list = tracks;
    untrack(() => {
      const present = new Set(list.map((t) => t.id));
      const kept = sel.ids.filter((id) => present.has(id));
      if (kept.length !== sel.ids.length) sel.set(kept);
    });
  });

  // listas longas montam em blocos, um por quadro: a navegação não espera as 1500 linhas (RNF-13)
  const chunk = new Chunked();
  $effect(() => chunk.advance(tracks.length));
  const rows = $derived(tracks.slice(0, chunk.shown));
  const totalMs = $derived(tracks.reduce((s, t) => s + t.durationMs, 0));
  const allSelected = $derived(tracks.length > 0 && sel.count === tracks.length);
  const queue = $derived(tracks.map((t) => t.id));
  const filtering = $derived(search.trim() !== '' || favoritesOnly);
</script>

<PageHeader title="Discoteca" eyebrow="Todas as faixas" subtitle={loading ? undefined : `${tracks.length} faixas · ${fmtDuration(totalMs)}`}>
  {#snippet actions()}
    <a class="btn" href={href(paths.youtube)}><Icon name="plus" size={16} />Adicionar músicas</a>
    <button class="btn btn-primary" disabled={!tracks.length} onclick={() => player.play(tracks[0].id, queue)}><Icon name="play" size={16} fill />Tocar tudo</button>
  {/snippet}
</PageHeader>

<TrackToolbar bind:search bind:sortBy bind:sortDir bind:favoritesOnly />

{#if sel.count > 0}
  <div class="sticky top-0 z-10 mb-3 flex flex-wrap items-center gap-2 rounded-lg border border-line-strong bg-s2 px-3 py-2" role="toolbar" aria-label="Ações para as faixas selecionadas">
    <span class="mr-auto text-sm font-medium">{sel.count} {sel.count === 1 ? 'selecionada' : 'selecionadas'}</span>
    <button class="btn btn-sm" onclick={async () => { await downloadTracks(sel.ids); sel.clear(); }}><Icon name="download" size={16} />Baixar</button>
    <button class="btn btn-sm" onclick={() => { ui.pickPlaylist([...sel.ids]); }}><Icon name="playlist" size={16} />Adicionar à playlist</button>
    <button class="btn btn-sm" onclick={async () => { if (await removeTracks(tracks.filter((t) => sel.has(t.id)))) sel.clear(); }}><Icon name="trash" size={16} />Remover</button>
    <button class="btn btn-sm" onclick={() => sel.clear()}>Limpar</button>
  </div>
{/if}

{#if loading}
  <SkeletonList variant="tracks" count={10} label="Carregando a discoteca" />
{:else if error}
  <State kind="error" text={error} onretry={() => res.reload()} />
{:else if tracks.length === 0}
  {#if filtering}
    <State kind="empty" icon="search" title="Nenhuma faixa encontrada" text="Ajuste a busca ou tire o filtro de favoritas." />
  {:else}
    <State kind="empty" title="Discoteca vazia" text="Adicione músicas pela busca do YouTube." />
  {/if}
{:else}
  <div class="mb-1 flex items-center justify-between gap-2 px-2 text-xs text-muted">
    <label class="flex items-center gap-2">
      <input type="checkbox" class="h-4 w-4 accent-[var(--lava)]" checked={allSelected} onchange={() => (allSelected ? sel.clear() : sel.set(queue))} />
      Selecionar tudo
    </label>
    <button class="flex items-center gap-2" role="switch" aria-checked={info} onclick={toggleInfo}>
      Mais informações
      <span class="relative h-5 w-9 rounded-full border border-line-strong transition-colors {info ? 'bg-[var(--lava)]' : 'bg-s3'}" aria-hidden="true">
        <span class="absolute top-0.5 h-3.5 w-3.5 rounded-full bg-fg transition-transform {info ? 'translate-x-[1.1rem]' : 'translate-x-0.5'}"></span>
      </span>
    </button>
  </div>
  <div role="list">
    {#each rows as t, i (t.id)}
      <TrackRow track={t} index={i} {info} selectable selected={sel.has(t.id)} onselect={() => sel.toggle(t.id)}
        current={player.track?.id === t.id} playing={player.isPlaying} onplay={() => player.play(t.id, queue)} ondelete={() => removeTracks([t])} />
    {/each}
  </div>
  {#if chunk.shown < tracks.length}<div aria-hidden="true" style="height:{chunk.rest(tracks.length)}px"></div>{/if}
{/if}
