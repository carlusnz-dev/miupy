<script lang="ts">
  // @req RF-18
  // Barra do player persistente (desktop). @req RF-01 RF-02 RF-03 RF-04
  import { player } from '../stores/player.svelte';
  import { href, paths } from '../stores/router.svelte';
  import { toggleFavorite } from '../actions';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';
  import Transport from './Transport.svelte';
  import Seek from './Seek.svelte';
  import Volume from './Volume.svelte';
  const t = $derived(player.track);
</script>

<footer class="motion-player grid h-[var(--player-h)] grid-cols-[1fr_minmax(20rem,2fr)_1fr] items-center gap-4 border-t border-line bg-s1 px-4" aria-label="Player">
  <div class="flex min-w-0 items-center gap-3">
    {#if t}
      <a href={href(paths.nowPlaying)} aria-label="Abrir tocando agora"><Cover path={t.coverPath} seed={t.id} size={56} rounded="0.375rem" /></a>
      <div class="min-w-0">
        <p class="truncate text-sm font-medium">{t.title}</p>
        {#if t.artistId}
          <a class="block truncate text-xs text-muted hover:underline" href={href(paths.artist(t.artistId))}>{t.artistName}</a>
        {:else}<p class="truncate text-xs text-muted">{t.artistName ?? 'Artista desconhecido'}</p>{/if}
        {#if player.state.streaming}<p class="text-xs text-lava-text">{player.isLoading ? 'Conectando…' : 'Streaming'}</p>{/if}
      </div>
      <button class="icon-btn motion-bounce {t.isFavorite ? 'on' : ''}" aria-pressed={t.isFavorite} aria-label={t.isFavorite ? 'Remover das favoritas' : 'Favoritar'} onclick={() => toggleFavorite(t)}>
        <Icon name="heart" size={18} fill={t.isFavorite} />
      </button>
    {:else}
      <p class="text-sm text-faint">Nada tocando</p>
    {/if}
  </div>

  <div class="flex flex-col items-center gap-0.5">
    <Transport />
    <div class="w-full"><Seek /></div>
  </div>

  <div class="flex items-center justify-end gap-1">
    <a class="icon-btn" href={href(paths.nowPlaying)} aria-label="Fila e letra"><Icon name="queue" size={20} /></a>
    <Volume />
  </div>
</footer>
