<script lang="ts">
  // Biblioteca: busca, ordenação, filtro de favoritas e seleção múltipla.
  // @req RF-05 RF-08 RF-10
  import { api, errorMessage, type SortDir, type Track, type TrackSort } from '../lib/ipc';
  import { library } from '../lib/stores/library.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { Selection } from '../lib/stores/selection.svelte';
  import { ui } from '../lib/stores/ui.svelte';
  import { downloadTracks } from '../lib/actions';
  import { fmtDuration } from '../lib/util/format';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import TrackToolbar from '../lib/components/TrackToolbar.svelte';
  import TrackRow from '../lib/components/TrackRow.svelte';
  import State from '../lib/components/State.svelte';
  import Icon from '../lib/components/Icon.svelte';

  let search = $state('');
  let sortBy = $state<TrackSort>('addedAt');
  let sortDir = $state<SortDir>('desc');
  let favoritesOnly = $state(false);
  let tracks = $state<Track[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  const sel = new Selection();

  async function load() {
    try {
      tracks = await api.libraryListTracks({ query: { search: search.trim() || undefined, sortBy, sortDir, favoritesOnly } });
      error = null;
      sel.set(sel.ids.filter((id) => tracks.some((t) => t.id === id)));
    } catch (e) { error = errorMessage(e); } finally { loading = false; }
  }
  // recarrega quando muda a consulta (com debounce na busca) ou chega library://changed
  $effect(() => {
    library.version; sortBy; sortDir; favoritesOnly;
    const q = search;
    const t = setTimeout(load, q ? 200 : 0);
    return () => clearTimeout(t);
  });

  const allSelected = $derived(tracks.length > 0 && sel.count === tracks.length);
  const queue = $derived(tracks.map((t) => t.id));
  const filtering = $derived(search.trim() !== '' || favoritesOnly);
</script>

<PageHeader title="Biblioteca" eyebrow="Todas as faixas" subtitle={loading ? undefined : `${tracks.length} faixas · ${fmtDuration(tracks.reduce((s, t) => s + t.durationMs, 0))}`}>
  {#snippet actions()}
    <button class="btn btn-primary" disabled={!tracks.length} onclick={() => player.play(tracks[0].id, queue)}><Icon name="play" size={16} fill />Tocar tudo</button>
  {/snippet}
</PageHeader>

<TrackToolbar bind:search bind:sortBy bind:sortDir bind:favoritesOnly />

{#if sel.count > 0}
  <div class="sticky top-0 z-10 mb-3 flex flex-wrap items-center gap-2 rounded-lg border border-line-strong bg-s2 px-3 py-2" role="toolbar" aria-label="Ações para as faixas selecionadas">
    <span class="mr-auto text-sm font-medium">{sel.count} {sel.count === 1 ? 'selecionada' : 'selecionadas'}</span>
    <button class="btn btn-sm" onclick={async () => { await downloadTracks(sel.ids); sel.clear(); }}><Icon name="download" size={16} />Baixar</button>
    <button class="btn btn-sm" onclick={() => { ui.pickPlaylist([...sel.ids]); }}><Icon name="playlist" size={16} />Adicionar à playlist</button>
    <button class="btn btn-sm" onclick={() => sel.clear()}>Limpar</button>
  </div>
{/if}

{#if loading}
  <State kind="loading" />
{:else if error}
  <State kind="error" text={error} onretry={load} />
{:else if tracks.length === 0}
  {#if filtering}
    <State kind="empty" icon="search" title="Nenhuma faixa encontrada" text="Ajuste a busca ou tire o filtro de favoritas." />
  {:else}
    <State kind="empty" title="Biblioteca vazia" text="Adicione músicas pela busca do YouTube." />
  {/if}
{:else}
  <label class="mb-1 flex items-center gap-2 px-2 text-xs text-muted">
    <input type="checkbox" class="h-4 w-4 accent-[var(--lava)]" checked={allSelected} onchange={() => (allSelected ? sel.clear() : sel.set(queue))} />
    Selecionar tudo
  </label>
  <div role="list">
    {#each tracks as t, i (t.id)}
      <TrackRow track={t} index={i} selectable selected={sel.has(t.id)} onselect={() => sel.toggle(t.id)}
        current={player.track?.id === t.id} playing={player.isPlaying} onplay={() => player.play(t.id, queue)} />
    {/each}
  </div>
{/if}
