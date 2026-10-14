<script lang="ts">
  // @req RF-18 RNF-10
  // Linha de faixa: capa, título/artista, álbum, duração e ações. @req RF-04 RF-05 RF-10
  import type { Track } from '../ipc';
  import { player } from '../stores/player.svelte';
  import { downloads } from '../stores/downloads.svelte';
  import { href, paths } from '../stores/router.svelte';
  import { downloadTracks, toggleFavorite } from '../actions';
  import { fmtTime } from '../util/format';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';
  import { scores, fmtScore, scoreTooltip } from '../stores/scores.svelte';
  import { useShared } from '../layoutPresence';
  $effect(() => { void scores.load([track.id]); }); // @req RF-25

  type Props = {
    track: Track; index?: number; current?: boolean; playing?: boolean;
    selectable?: boolean; selected?: boolean; onselect?: () => void;
    onplay: () => void;
    extra?: import('svelte').Snippet;
    draggable?: boolean;
    ondragstart?: (e: DragEvent) => void; ondragover?: (e: DragEvent) => void; ondrop?: (e: DragEvent) => void;
  };
  let { track, index, current = false, playing = false, selectable = false, selected = false, onselect, onplay, extra, draggable = false, ondragstart, ondragover, ondrop }: Props = $props();

  const job = $derived(downloads.forTrack(track.id));
  const dl = $derived(job && (job.status === 'queued' || job.status === 'downloading') ? job : null);
</script>

<div
  class="group motion-item track-row flex items-center gap-2 rounded-lg px-2 py-1.5 hover:bg-s2 {current ? 'bg-s2' : ''} {selected ? 'bg-s3' : ''}"
  {draggable} {ondragstart} {ondragover} {ondrop} role="listitem"
>
  {#if selectable}
    <input type="checkbox" class="h-4 w-4 shrink-0 accent-[var(--lava)]" checked={selected} onchange={onselect} aria-label="Selecionar {track.title}" />
  {/if}
  {#if draggable}
    <span class="hidden cursor-grab text-faint md:block" aria-hidden="true"><Icon name="grip" size={16} /></span>
  {/if}
  {#if index !== undefined}
    <span class="tnum hidden w-6 shrink-0 text-right text-xs text-faint md:block">{current && playing ? '♪' : index + 1}</span>
  {/if}

  <button class="motion-bounce flex min-w-0 flex-1 items-center gap-3 rounded-md text-left" onclick={onplay} aria-label="Tocar {track.title}{track.artistName ? `, de ${track.artistName}` : ''}">
    <span class="relative shrink-0">
      <Cover path={track.coverPath} seed={track.id} size={44} rounded="0.375rem" />
      <span class="absolute inset-0 grid place-items-center rounded-md bg-black/50 text-fg opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
        <Icon name="play" size={18} />
      </span>
    </span>
    <span class="min-w-0">
      <span class="block truncate text-sm font-medium {current ? 'text-lava-text' : ''}">{track.title}</span>
      <span class="block truncate text-xs text-muted" use:useShared={track.artistId ? `artist-title-${track.artistId}` : ''}>{track.artistName ?? 'Artista desconhecido'}{#if current && player.state.streaming} · {player.isLoading ? 'Conectando…' : 'Streaming'}{:else if track.source === 'youtube' && !track.filePath} · Disponível por streaming{/if}</span>
    </span>
  </button>

  <span class="hidden min-w-0 flex-1 truncate text-sm text-muted lg:block">
    {#if track.artistId && track.album}<a class="hover:underline" href={href(paths.artist(track.artistId))}>{track.album}</a>{:else}{track.album ?? '—'}{/if}
  </span>

  <button class="icon-btn motion-bounce {track.isFavorite ? 'on' : ''}" aria-pressed={track.isFavorite} aria-label={track.isFavorite ? `Remover ${track.title} das favoritas` : `Favoritar ${track.title}`} onclick={() => toggleFavorite(track)}>
    <Icon name="heart" size={18} fill={track.isFavorite} />
  </button>

  <span class="flex w-9 shrink-0 justify-center">
    {#if dl}
      <span class="tnum text-xs text-lava-text" role="status" aria-label="Baixando {Math.round(dl.progress * 100)}%">{dl.status === 'queued' ? '…' : `${Math.round(dl.progress * 100)}%`}</span>
    {:else if track.downloadStatus === 'done'}
      <span class="text-ok" title="Baixada" role="img" aria-label="Baixada"><Icon name="check" size={16} /></span>
    {:else}
      <button class="icon-btn" aria-label={track.downloadStatus === 'error' ? `Tentar baixar ${track.title} de novo` : `Baixar ${track.title}`} onclick={() => downloadTracks([track.id])}>
        <Icon name={track.downloadStatus === 'error' ? 'alert' : 'download'} size={18} class={track.downloadStatus === 'error' ? 'text-danger' : ''} />
      </button>
    {/if}
  </span>

  {#if scores.get(track.id)}<span class="tnum hidden w-10 text-right text-xs text-faint sm:block" title={scoreTooltip(scores.get(track.id))} aria-label={`Pontuação ${fmtScore(scores.get(track.id).score)}`}>{fmtScore(scores.get(track.id).score)}</span>{/if}
  <span class="tnum w-11 shrink-0 text-right text-xs text-muted">{fmtTime(track.durationMs)}</span>
  {#if extra}{@render extra()}{/if}
</div>

<style>
  /* Só as primeiras linhas (as que cabem na tela) fazem a entrada em mola; as de baixo, montadas em
     blocos ou ao rolar, entram sem animar (RNF-13: centenas de animações simultâneas engasgam). */
  .track-row:nth-child(n + 15) { animation: none; }
</style>
