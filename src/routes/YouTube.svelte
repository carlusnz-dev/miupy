<script lang="ts">
  // Busca no YouTube: resultados, adicionar à discoteca/playlist, baixar. Com a caixa vazia, buscas
  // recentes e recomendações. @req RF-09 RF-10 RF-34
  import { api, type SearchResult } from '../lib/ipc';
  import { ui } from '../lib/stores/ui.svelte';
  import { notify, notifyError } from '../lib/stores/toast.svelte';
  import { downloadTracks } from '../lib/actions';
  import { fmtTime } from '../lib/util/format';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import Cover from '../lib/components/Cover.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import State from '../lib/components/State.svelte';
  import SkeletonList from '../lib/components/SkeletonList.svelte';
  import Recommendations from '../lib/components/Recommendations.svelte';
  import { YoutubeSearchStore } from '../lib/stores/youtubeSearch.svelte';
  import { SearchHistoryStore } from '../lib/stores/searchHistory.svelte';
  import { SuggestionsStore } from '../lib/stores/suggestions.svelte';

  let query = $state('');
  const suggestions = new SuggestionsStore();
  $effect(() => { suggestions.update(query); return () => suggestions.dispose(); }); // @req RF-23
  function pick(text: string) { suggestions.close(text); query = text; void search(); }
  function onSearchKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') { e.preventDefault(); suggestions.move(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); suggestions.move(-1); }
    else if (e.key === 'Escape') suggestions.close();
    else if (e.key === 'Enter' && suggestions.open && suggestions.active >= 0) { e.preventDefault(); pick(suggestions.items[suggestions.active]); }
  }
  // @req RF-34
  const history = new SearchHistoryStore();
  const emptyBox = $derived(!query.trim());
  $effect(() => { if (emptyBox) { found.reset(); void history.load(); } });
  function repeat(text: string) { query = text; void search(); }
  async function clearHistory() {
    try { await history.clear(); } catch (e) { notifyError(e); }
  }
  const found = new YoutubeSearchStore();
  let busy = $state<string | null>(null);

  async function search(e?: SubmitEvent) {
    e?.preventDefault();
    const q = query.trim();
    if (!q) return;
    suggestions.close(q);
    await found.run(q);
  }

  async function add(r: SearchResult, then?: 'download' | 'playlist') {
    busy = r.sourceId;
    try {
      const tracks = await api.youtubeAddToLibrary({ sourceIds: [r.sourceId] });
      found.results = found.results?.map((x) => (x.sourceId === r.sourceId ? { ...x, inLibrary: true, trackId: tracks[0]?.id ?? null, isFavorite: tracks[0]?.isFavorite ?? false } : x)) ?? null;
      if (then === 'download') await downloadTracks(tracks.map((t) => t.id));
      else if (then === 'playlist') ui.pickPlaylist(tracks.map((t) => t.id));
      else notify('Adicionada à discoteca.');
    } catch (e) { notifyError(e); } finally { busy = null; }
  }
  // @req RF-04
  async function toggleFavorite(r: SearchResult) {
    busy = r.sourceId;
    try {
      const trackId = r.trackId ?? (await api.youtubeAddToLibrary({ sourceIds: [r.sourceId] }))[0]?.id;
      if (!trackId) return;
      const track = await api.librarySetFavorite({ trackId, favorite: !r.isFavorite });
      found.results = found.results?.map((x) => x.sourceId === r.sourceId
        ? { ...x, inLibrary: true, trackId, isFavorite: track.isFavorite } : x) ?? null;
    } catch (e) { notifyError(e); }
    finally { busy = null; }
  }
</script>

<PageHeader title="Buscar no YouTube" subtitle="Encontre músicas, adicione à discoteca e baixe para ouvir offline." />

<form class="mb-5 flex gap-2" onsubmit={search} role="search">
  <div class="relative flex-1">
    <Icon name="search" size={16} class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-faint" />
    <input class="field !pl-9" type="search" placeholder="Nome da música ou artista" aria-label="Buscar no YouTube" bind:value={query} onkeydown={onSearchKey} role="combobox" aria-autocomplete="list" aria-expanded={suggestions.open} aria-controls="yt-suggestions" aria-activedescendant={suggestions.open && suggestions.active >= 0 ? `yt-suggestion-${suggestions.active}` : undefined} />
    {#if suggestions.open}<ul id="yt-suggestions" role="listbox" aria-label="Sugestões de busca" class="glass-strong absolute z-20 mt-1 w-full rounded-lg p-1">
      {#each suggestions.items as item, i}<li id="yt-suggestion-{i}" role="option" aria-selected={suggestions.active === i}><button type="button" class="w-full rounded px-3 py-2 text-left text-sm hover:bg-s3 {suggestions.active === i ? 'bg-s3' : ''}" onpointerdown={(e) => e.preventDefault()} onclick={() => pick(item)}>{item}</button></li>{/each}
    </ul>{/if}
  </div>
  <button class="btn btn-primary" disabled={found.searching || !query.trim()}>Buscar</button>
</form>

{#if emptyBox}
  <section aria-labelledby="recent-searches-heading">
    <div class="mb-3 flex items-center justify-between gap-3">
      <h2 id="recent-searches-heading" class="text-lg font-semibold">Buscas recentes</h2>
      {#if history.items.length}<button class="btn btn-sm" onclick={clearHistory}><Icon name="trash" size={15} />Limpar histórico</button>{/if}
    </div>
    {#if !history.ready}<SkeletonList variant="tracks" count={3} label="Carregando buscas recentes" />
    {:else if history.error}<p class="text-sm text-danger" role="alert">{history.error}</p>
    {:else if !history.items.length}<State kind="empty" icon="search" title="Nenhuma busca ainda" text="As músicas que você buscar aparecem aqui para repetir com um toque. A busca precisa de internet." />
    {:else}
      <ul class="flex flex-wrap gap-2">
        {#each history.items as item (item)}
          <li><button class="chip hover:bg-s3" onclick={() => repeat(item)}><Icon name="search" size={12} />{item}</button></li>
        {/each}
      </ul>
    {/if}
  </section>
  <Recommendations />
{:else if found.searching}
  <SkeletonList variant="tracks" count={6} label="Buscando no YouTube" />
{:else if found.error}
  <State kind="error" title="A busca falhou" text={found.error} onretry={search} />
{:else if found.results === null}
  <State kind="empty" icon="globe" title="Busque uma música" text="A busca precisa de internet. O que você baixar toca sem rede." />
{:else if found.results.length === 0}
  <State kind="empty" icon="search" title="Nada encontrado" text="Tente outro termo." />
{:else}
  <ul class="flex flex-col gap-1">
    {#each found.results as r (r.sourceId)}
      <li class="flex flex-wrap items-center gap-3 rounded-lg px-2 py-2 hover:bg-s2">
        <Cover url={r.thumbnailUrl} seed={r.sourceId} size={52} rounded="0.375rem" />
        <div class="min-w-0 flex-1 basis-40">
          <p class="truncate text-sm font-medium">{r.title}</p>
          <p class="truncate text-xs text-muted">{r.artist} · {fmtTime(r.durationMs)}</p>
        </div>
        {#if r.inLibrary}<span class="chip text-ok"><Icon name="check" size={12} />Na discoteca</span>{/if}
        <div class="flex gap-1.5">
          <button class="icon-btn" aria-label={r.isFavorite ? `Desfavoritar ${r.title}` : `Favoritar ${r.title}`}
            aria-pressed={r.isFavorite} disabled={busy === r.sourceId} onclick={() => toggleFavorite(r)}>
            <Icon name="heart" size={18} fill={r.isFavorite} />
          </button>
          {#if !r.inLibrary}
            <button class="btn btn-sm" disabled={busy === r.sourceId} onclick={() => add(r)}><Icon name="plus" size={14} />Adicionar</button>
          {/if}
          <button class="btn btn-sm" disabled={busy === r.sourceId} onclick={() => add(r, 'playlist')}><Icon name="playlist" size={14} />Playlist</button>
          <button class="btn btn-sm" disabled={busy === r.sourceId} onclick={() => add(r, 'download')}><Icon name="download" size={14} />Baixar</button>
        </div>
      </li>
    {/each}
  </ul>
{/if}
