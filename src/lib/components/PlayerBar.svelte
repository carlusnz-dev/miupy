<script lang="ts">
  // @req RF-18
  // Barra do player persistente (desktop). @req RF-01 RF-02 RF-03 RF-04
  import { player } from '../stores/player.svelte';
  import { href, paths } from '../stores/router.svelte';
  import { toggleFavorite } from '../actions';
  import { favorites } from '../stores/favorites.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';
  import Transport from './Transport.svelte';
  import Seek from './Seek.svelte';
  import Volume from './Volume.svelte';
  import ShuffleButton from './ShuffleButton.svelte';
  import { useShared } from '../layoutPresence';
  const t = $derived(player.track);
</script>

<footer class="motion-player grid h-[var(--player-h)] grid-cols-[1fr_minmax(20rem,2fr)_1fr] items-center gap-4 border-t border-line bg-s1 px-4" aria-label="Player">
  <div class="flex min-w-0 items-center gap-3">
    {#if t}
      <a href={href(paths.nowPlaying)} aria-label="Abrir tocando agora"><span class="relative block"><Cover path={t.coverPath} thumb={t.thumbnailUrl} seed={t.id} size={56} rounded="0.375rem" shared={`track-cover-${t.id}`} />{#if player.isLoading}<span class="absolute inset-0 grid place-items-center rounded-md bg-black/60 text-xs text-white" role="status">…</span>{/if}</span></a>
      <div class="min-w-0">
        <p class="truncate text-sm font-medium" use:useShared={`track-title-${t.id}`}>{t.title}</p>
        {#if t.artistId}
          <a class="block truncate text-xs text-muted hover:underline" href={href(paths.artist(t.artistId))} use:useShared={`track-artist-${t.id}`}>{t.artistName}</a>
        {:else}<p class="truncate text-xs text-muted" use:useShared={`track-artist-${t.id}`}>{t.artistName ?? 'Artista desconhecido'}</p>{/if}
        {#if player.state.streaming}<p class="text-xs text-lava-text">{player.isLoading ? 'Conectando…' : 'Streaming'}</p>{/if}
      </div>
      <button class="icon-btn motion-bounce {favorites.is(t) ? 'on' : ''}" aria-pressed={favorites.is(t)} aria-label={favorites.is(t) ? 'Remover das favoritas' : 'Favoritar'} onclick={() => toggleFavorite(t)}>
        <Icon name="heart" size={18} fill={favorites.is(t)} />
      </button>
    {:else}
      <p class="text-sm text-faint">Nada tocando</p>
    {/if}
  </div>

  <div class="flex flex-col items-center gap-0.5">
    <div class="flex items-center"><ShuffleButton /><Transport /></div>
    <div class="w-full"><Seek /></div>
  </div>

  <div class="flex items-center justify-end gap-1">
    <a class="icon-btn" href={href(paths.nowPlaying)} aria-label="Fila e letra"><Icon name="queue" size={20} /></a>
    <Volume />
  </div>
</footer>
