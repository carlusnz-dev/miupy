<script lang="ts">
  // @req RF-19
  // Detalhe da playlist: faixas, busca/ordenação, reordenar, remover, baixar tudo.
  // @req RF-06 RF-07 RF-08 RF-11 RNF-11 RNF-13
  import { untrack } from 'svelte';
  import { api, type Playlist, type SortDir, type Track, type TrackSort } from '../lib/ipc';
  import { library } from '../lib/stores/library.svelte';
  import { Resource } from '../lib/stores/resource.svelte';
  import { mergeList } from '../lib/util/diff';
  import { Chunked } from '../lib/util/chunked.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { Selection } from '../lib/stores/selection.svelte';
  import { navigate, href, paths } from '../lib/stores/router.svelte';
  import { ui } from '../lib/stores/ui.svelte';
  import { notify, notifyError } from '../lib/stores/toast.svelte';
  import { downloadPlaylist, downloadTracks } from '../lib/actions';
  import { fmtDuration } from '../lib/util/format';
  import TrackToolbar from '../lib/components/TrackToolbar.svelte';
  import TrackRow from '../lib/components/TrackRow.svelte';
  import PlaylistForm from '../lib/components/PlaylistForm.svelte';
  import PlaylistCover from '../lib/components/PlaylistCover.svelte';
  import { readImageUpload } from '../lib/util/imageUpload';
  import State from '../lib/components/State.svelte';
  import Skeleton from '../lib/components/Skeleton.svelte';
  import SkeletonList from '../lib/components/SkeletonList.svelte';
  import Recommendations from '../lib/components/Recommendations.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import MeshGradient from '../lib/components/MeshGradient.svelte';
  import { useShared } from '../lib/layoutPresence';

  let { id }: { id: string } = $props();

  let search = $state('');
  let sortBy = $state<TrackSort>('position');
  let sortDir = $state<SortDir>('asc');
  let favoritesOnly = $state(false);
  let editing = $state<{ playlist: Playlist | null } | null>(null);
  const sel = new Selection();

  // O cabeçalho vem das playlists em cache na store; só a lista de faixas tem consulta própria, com
  // cache por consulta (RNF-11). library://changed troca apenas as linhas que mudaram (RNF-13).
  const res = new Resource<Track[]>(['tracks'], mergeList((t: Track) => t.id));
  const tracks = $derived(res.data ?? []);
  const loading = $derived(res.loading);
  const error = $derived(res.data === undefined ? res.error : null);
  $effect(() => res.attach());

  const pl = $derived(library.playlists.find((p) => p.id === id));
  const canReorder = $derived(sortBy === 'position' && !search.trim() && !favoritesOnly);
  const queue = $derived(tracks.map((t) => t.id));
  const chunk = new Chunked();
  $effect(() => chunk.advance(tracks.length));
  const rows = $derived(tracks.slice(0, chunk.shown));

  $effect(() => {
    sortBy; sortDir; favoritesOnly;
    const q = search.trim();
    const query = { search: q || undefined, sortBy, sortDir, favoritesOnly };
    const run = () => { void res.load(`playlist:${id}:${JSON.stringify(query)}`, () => api.playlistGetTracks({ playlistId: id, query })); };
    if (!q) { run(); return; }
    const t = setTimeout(run, 200);
    return () => clearTimeout(t);
  });
  $effect(() => {
    const list = tracks;
    untrack(() => {
      const present = new Set(list.map((t) => t.id));
      const kept = sel.ids.filter((x) => present.has(x));
      if (kept.length !== sel.ids.length) sel.set(kept);
    });
  });

  async function remove(ids: string[]) {
    try { await api.playlistRemoveTracks({ playlistId: id, trackIds: ids }); sel.clear(); notify('Removida da playlist.'); } catch (e) { notifyError(e); }
  }
  async function move(trackId: string, newPosition: number) {
    if (newPosition < 0 || newPosition >= tracks.length) return;
    try { await api.playlistMoveTrack({ playlistId: id, trackId, newPosition }); } catch (e) { notifyError(e); }
  }
  // @req RF-30
  async function markFavorites() {
    if (!pl || pl.isFavorites) return;
    try { await api.favoritesSetPlaylist({ playlistId: id }); await library.reloadPlaylists(); notify('Playlist marcada como favoritas.'); }
    catch (e) { notifyError(e); }
  }
  async function destroy() {
    if (!pl || !(await ui.confirm('Apagar playlist', `"${pl.name}" será apagada. As faixas continuam na discoteca.${pl.isFavorites ? " As favoritas ficarão sem playlist até você marcar outra ou favoritar uma faixa." : ""}`, 'Apagar', true))) return;
    try { await api.playlistDelete({ playlistId: id }); navigate(paths.playlists); } catch (e) { notifyError(e); }
  }

  // arrastar e soltar (desktop; no toque, use os botões subir/descer)
  let coverInput = $state<HTMLInputElement>();
  async function setCover(file: File | null) { // @req RF-22
    try { await api.playlistSetCover({ playlistId: id, image: file ? await readImageUpload(file) : null }); await library.reloadPlaylists(); notify(file ? 'Capa atualizada.' : 'Capa removida.'); }
    catch (e) { notifyError(e); }
  }
  let dragId = $state<string | null>(null);
</script>

{#if !pl && error}
  <State kind="error" title="Playlist não encontrada" text={error} />
{:else}
  <MeshGradient class="mb-5 rounded-2xl border border-line" intensity={0.8}>
    <div class="flex flex-col items-start gap-4 p-4 sm:flex-row sm:items-end md:p-6">
      {#if pl}<PlaylistCover playlist={pl} size={144} shared={`playlist-cover-${pl.id}`} />{:else}<Skeleton w="144px" h="144px" rounded="0.75rem" />{/if}
      <div class="min-w-0 flex-1">
        <p class="eyebrow !text-fg/70">{pl?.isFavorites ? "Playlist de favoritas" : "Playlist"}</p>
        {#if pl}
          <h1 class="truncate text-3xl font-bold tracking-tight md:text-4xl" use:useShared={`playlist-title-${pl.id}`}>{pl.name}</h1>
        {:else}
          <Skeleton w="min(18rem, 70%)" h="2.25rem" class="my-1" />
        {/if}
        {#if pl?.description}<p class="mt-1 text-sm text-fg/80">{pl.description}</p>{/if}
        {#if pl}<p class="mt-1 text-sm text-fg/70">{pl.trackCount} faixas · {fmtDuration(pl.totalDurationMs)}</p>{/if}
      </div>
    </div>
  </MeshGradient>

  <div class="mb-4 flex flex-wrap items-center gap-2">
    <button class="btn btn-primary" disabled={!tracks.length} onclick={() => player.play(tracks[0].id, queue)}><Icon name="play" size={16} fill />Tocar</button>
    <button class="btn" disabled={!tracks.length} onclick={() => downloadPlaylist(id)}><Icon name="download" size={16} />Baixar playlist</button>
    <button class="btn" onclick={() => (editing = { playlist: pl ?? null })} disabled={!pl}><Icon name="edit" size={16} />Renomear</button>
    <input bind:this={coverInput} type="file" accept="image/jpeg,image/png,image/webp" class="sr-only" aria-label="Escolher capa" onchange={(e) => { const file = e.currentTarget.files?.[0]; if (file) void setCover(file); e.currentTarget.value = ''; }} />
    <button class="btn" disabled={!pl} onclick={() => coverInput?.click()}>Escolher capa</button>
    {#if pl?.coverPath}<button class="btn" onclick={() => setCover(null)}>Remover capa</button>{/if}
    {#if pl && !pl.isFavorites}<button class="btn" onclick={markFavorites}><Icon name="heart" size={16} />Marcar como favoritas</button>{/if}
    <button class="btn btn-danger" onclick={destroy} disabled={!pl}><Icon name="trash" size={16} />Apagar</button>
    <a class="btn ml-auto" href={href(paths.playlists)}><Icon name="left" size={16} />Playlists</a>
  </div>

  <TrackToolbar bind:search bind:sortBy bind:sortDir bind:favoritesOnly allowPosition placeholder="Buscar nesta playlist" />

  {#if sel.count > 0}
    <div class="sticky top-0 z-10 mb-3 flex flex-wrap items-center gap-2 rounded-lg border border-line-strong bg-s2 px-3 py-2" role="toolbar" aria-label="Ações para as faixas selecionadas">
      <span class="mr-auto text-sm font-medium">{sel.count} {sel.count === 1 ? 'selecionada' : 'selecionadas'}</span>
      <button class="btn btn-sm" onclick={async () => { await downloadTracks(sel.ids); sel.clear(); }}><Icon name="download" size={16} />Baixar</button>
      <button class="btn btn-sm" onclick={() => ui.pickPlaylist([...sel.ids])}><Icon name="playlist" size={16} />Outra playlist</button>
      <button class="btn btn-sm btn-danger" onclick={() => remove([...sel.ids])}><Icon name="trash" size={16} />Remover</button>
      <button class="btn btn-sm" onclick={() => sel.clear()}>Limpar</button>
    </div>
  {/if}

  {#if loading}
    <SkeletonList variant="tracks" count={8} label="Carregando as faixas da playlist" />
  {:else if error}
    <State kind="error" text={error} onretry={() => res.reload()} />
  {:else if tracks.length === 0}
    {#if search.trim() || favoritesOnly}
      <State kind="empty" icon="search" title="Nenhuma faixa encontrada" text="Ajuste a busca ou o filtro." />
    {:else}
      <State kind="empty" title="Playlist vazia" text="Adicione faixas pela discoteca (seleção múltipla) ou pela busca do YouTube.">
        <a class="btn btn-primary" href={href(paths.library)}>Ir para a discoteca</a>
      </State>
    {/if}
  {:else}
    <div role="list">
      {#each rows as t, i (t.id)}
        <TrackRow track={t} index={i} selectable selected={sel.has(t.id)} onselect={() => sel.toggle(t.id)}
          current={player.track?.id === t.id} playing={player.isPlaying} onplay={() => player.play(t.id, queue)}
          draggable={canReorder}
          ondragstart={(e) => { dragId = t.id; e.dataTransfer?.setData('text/plain', t.id); }}
          ondragover={(e) => { if (dragId) e.preventDefault(); }}
          ondrop={(e) => { e.preventDefault(); if (dragId && dragId !== t.id) void move(dragId, i); dragId = null; }}>
          {#snippet extra()}
            {#if canReorder}
              <span class="flex">
                <button class="icon-btn !h-8 !w-8" aria-label="Subir {t.title}" disabled={i === 0} onclick={() => move(t.id, i - 1)}><Icon name="up" size={16} /></button>
                <button class="icon-btn !h-8 !w-8" aria-label="Descer {t.title}" disabled={i === tracks.length - 1} onclick={() => move(t.id, i + 1)}><Icon name="down" size={16} /></button>
              </span>
            {/if}
            <button class="icon-btn !h-8 !w-8" aria-label="Remover {t.title} da playlist" onclick={() => remove([t.id])}><Icon name="close" size={16} /></button>
          {/snippet}
        </TrackRow>
      {/each}
    </div>
    {#if chunk.shown < tracks.length}<div aria-hidden="true" style="height:{chunk.rest(tracks.length)}px"></div>{/if}
    {#if !canReorder}<p class="mt-3 text-xs text-faint">Para reordenar, ordene por "Ordem da playlist" e limpe a busca e o filtro.</p>{/if}
  {/if}
{/if}

<PlaylistForm state={editing} onclose={() => (editing = null)} />

<Recommendations playlistId={id} />
