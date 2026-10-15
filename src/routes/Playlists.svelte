<script lang="ts">
  // Lista de playlists: criar, renomear, apagar. @req RF-06 RNF-11
  import { api, type Playlist } from '../lib/ipc';
  import { library } from '../lib/stores/library.svelte';
  import { href, paths } from '../lib/stores/router.svelte';
  import { ui } from '../lib/stores/ui.svelte';
  import { notify, notifyError } from '../lib/stores/toast.svelte';
  import { fmtDuration } from '../lib/util/format';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import PlaylistCover from '../lib/components/PlaylistCover.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import State from '../lib/components/State.svelte';
  import SkeletonList from '../lib/components/SkeletonList.svelte';
  import PlaylistForm from '../lib/components/PlaylistForm.svelte';
  import { useShared } from '../lib/layoutPresence';

  // As playlists ficam em cache na própria store (carregadas na abertura do app e atualizadas por
  // library://changed): a tela mostra o que há e só usa o esqueleto se ainda não chegou nada.
  const loading = $derived(!library.playlistsReady && !library.playlistsError);
  const error = $derived(library.playlistsError);
  let form = $state<{ playlist: Playlist | null } | null>(null);

  async function remove(p: Playlist) {
    if (!(await ui.confirm('Apagar playlist', `"${p.name}" será apagada. As faixas continuam na discoteca.${p.isFavorites ? " As favoritas ficarão sem playlist até você marcar outra ou favoritar uma faixa." : ""}`, 'Apagar', true))) return;
    try { await api.playlistDelete({ playlistId: p.id }); notify('Playlist apagada.'); } catch (e) { notifyError(e); }
  }
</script>

<PageHeader title="Playlists" subtitle={loading ? undefined : `${library.playlists.length} playlists`}>
  {#snippet actions()}
    <button class="btn btn-primary" onclick={() => (form = { playlist: null })}><Icon name="plus" size={16} />Nova playlist</button>
  {/snippet}
</PageHeader>

{#if loading}
  <SkeletonList variant="cards" count={6} label="Carregando as playlists" />
{:else if error}
  <State kind="error" text={error} onretry={() => library.reloadPlaylists()} />
{:else if library.playlists.length === 0}
  <State kind="empty" icon="playlist" title="Nenhuma playlist ainda" text="Crie uma para organizar suas faixas.">
    <button class="btn btn-primary" onclick={() => (form = { playlist: null })}>Criar playlist</button>
  </State>
{:else}
  <ul class="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-4">
    {#each library.playlists as p (p.id)}
      <li class="card group relative p-3 hover:bg-s2">
        <a href={href(paths.playlist(p.id))} class="block" aria-label="Abrir playlist {p.name}">
          <PlaylistCover playlist={p} class="mb-3 w-full" shared={`playlist-cover-${p.id}`} />
          <span class="block truncate font-semibold" use:useShared={`playlist-title-${p.id}`}>{p.name}</span>
          {#if p.isFavorites}<span class="chip text-primary"><Icon name="heart" size={12} fill />Favoritas</span>{/if}
          <span class="block text-xs text-muted">{p.trackCount} faixas · {fmtDuration(p.totalDurationMs)}</span>
          {#if p.description}<span class="mt-1 line-clamp-2 block text-xs text-faint">{p.description}</span>{/if}
        </a>
        <div class="mt-2 flex justify-end gap-1">
          <button class="icon-btn" aria-label="Renomear {p.name}" onclick={() => (form = { playlist: p })}><Icon name="edit" size={16} /></button>
          <button class="icon-btn" aria-label="Apagar {p.name}" onclick={() => remove(p)}><Icon name="trash" size={16} /></button>
        </div>
      </li>
    {/each}
  </ul>
{/if}

<PlaylistForm state={form} onclose={() => (form = null)} />
