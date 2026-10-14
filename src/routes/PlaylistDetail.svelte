<script lang="ts">
  // @req RF-19
  // Detalhe da playlist: faixas, busca/ordenação, reordenar, remover, baixar tudo.
  // @req RF-06 RF-07 RF-08 RF-11
  import { api, errorMessage, type Playlist, type SortDir, type Track, type TrackSort } from '../lib/ipc';
  import { library } from '../lib/stores/library.svelte';
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
  import Recommendations from '../lib/components/Recommendations.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import MeshGradient from '../lib/components/MeshGradient.svelte';
  import { useShared } from '../lib/layoutPresence';

  let { id }: { id: string } = $props();

  let search = $state('');
  let sortBy = $state<TrackSort>('position');
  let sortDir = $state<SortDir>('asc');
  let favoritesOnly = $state(false);
  let tracks = $state<Track[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let editing = $state<{ playlist: Playlist | null } | null>(null);
  const sel = new Selection();

  const pl = $derived(library.playlists.find((p) => p.id === id));
  const canReorder = $derived(sortBy === 'position' && !search.trim() && !favoritesOnly);
  const queue = $derived(tracks.map((t) => t.id));

  async function load() {
    try {
      tracks = await api.playlistGetTracks({ playlistId: id, query: { search: search.trim() || undefined, sortBy, sortDir, favoritesOnly } });
      error = null;
      sel.set(sel.ids.filter((x) => tracks.some((t) => t.id === x)));
    } catch (e) { error = errorMessage(e); } finally { loading = false; }
  }
  $effect(() => {
    library.version; sortBy; sortDir; favoritesOnly;
    const q = search;
    const t = setTimeout(load, q ? 200 : 0);
    return () => clearTimeout(t);
  });

  async function remove(ids: string[]) {
    try { await api.playlistRemoveTracks({ playlistId: id, trackIds: ids }); sel.clear(); notify('Removida da playlist.'); } catch (e) { notifyError(e); }
  }
  async function move(trackId: string, newPosition: number) {
    if (newPosition < 0 || newPosition >= tracks.length) return;
    try { await api.playlistMoveTrack({ playlistId: id, trackId, newPosition }); } catch (e) { notifyError(e); }
  }
  async function destroy() {
    if (!pl || !(await ui.confirm('Apagar playlist', `"${pl.name}" será apagada. As faixas continuam na discoteca.`, 'Apagar', true))) return;
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

{#if !pl && !loading && error}
  <State kind="error" title="Playlist não encontrada" text={error} />
{:else}
  <MeshGradient class="mb-5 rounded-2xl border border-line" intensity={0.8}>
    <div class="flex flex-col items-start gap-4 p-4 sm:flex-row sm:items-end md:p-6">
      {#if pl}<PlaylistCover playlist={pl} size={144} shared={`playlist-cover-${pl.id}`} />{/if}
      <div class="min-w-0 flex-1">
        <p class="eyebrow !text-fg/70">Playlist</p>
        <h1 class="truncate text-3xl font-bold tracking-tight md:text-4xl" use:useShared={pl ? `playlist-title-${pl.id}` : ''}>{pl?.name ?? '…'}</h1>
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
    <State kind="loading" />
  {:else if error}
    <State kind="error" text={error} onretry={load} />
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
      {#each tracks as t, i (t.id)}
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
    {#if !canReorder}<p class="mt-3 text-xs text-faint">Para reordenar, ordene por "Ordem da playlist" e limpe a busca e o filtro.</p>{/if}
  {/if}
{/if}

<PlaylistForm state={editing} onclose={() => (editing = null)} />

<Recommendations playlistId={id} />
