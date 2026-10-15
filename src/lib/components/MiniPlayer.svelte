<script lang="ts">
  // @req RF-18
  // Mini player (mobile), acima da navegação inferior. @req RF-01 RF-03
  import { springTransition } from '../motion';
  import { player } from '../stores/player.svelte';
  import { navigate, paths } from '../stores/router.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';
  import { useShared } from '../layoutPresence';
  const t = $derived(player.track);
</script>

{#if t}
  <div transition:springTransition class="relative border-t border-line bg-s2" role="region" aria-label="Mini player">
    <div class="absolute inset-x-0 top-0 h-0.5 bg-s3" aria-hidden="true"><div class="h-full" style="width:{player.progress * 100}%;background:var(--grad-lava)"></div></div>
    <div class="flex items-center gap-2 px-3 py-2">
      <button class="flex min-w-0 flex-1 items-center gap-3 text-left" onclick={() => navigate(paths.nowPlaying)} aria-label="Abrir tocando agora: {t.title}">
        <Cover path={t.coverPath} thumb={t.thumbnailUrl} seed={t.id} size={40} rounded="0.375rem" shared={`track-cover-${t.id}`} />
        <span class="min-w-0">
          <span class="block truncate text-sm font-medium" use:useShared={`track-title-${t.id}`}>{t.title}</span>
          <span class="block truncate text-xs text-muted" use:useShared={`track-artist-${t.id}`}>{t.artistName ?? 'Artista desconhecido'}{player.state.streaming ? ` · ${player.isLoading ? 'Conectando…' : 'Streaming'}` : ''}</span>
        </span>
      </button>
      <button class="icon-btn" aria-label={player.isPlaying ? 'Pausar' : 'Tocar'} onclick={() => player.toggle()}>
        <Icon name={player.isPlaying ? 'pause' : 'play'} size={24} fill />
      </button>
      <button class="icon-btn" aria-label="Próxima faixa" disabled={!player.hasNext} onclick={() => player.next()}><Icon name="next" size={22} /></button>
    </div>
  </div>
{/if}
