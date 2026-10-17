<script lang="ts">
  // @req RF-18
  // Mini player (mobile), acima da navegação inferior. @req RF-01 RF-03
  import { springTransition } from '../motion';
  import { player } from '../stores/player.svelte';
  import { navigate, paths } from '../stores/router.svelte';
  import { beginDrag } from '../util/drag';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';
  const t = $derived(player.track);
  let button = $state<HTMLButtonElement>();
  let drag: { pointerId: number; session: ReturnType<typeof beginDrag> } | null = null;
  let offset = $state(0);
  let moving = $state(false);
  let swiped = false;
  const reduced = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  // @req RF-46
  function pointerDown(event: PointerEvent) {
    if (event.button !== 0) return;
    swiped = false;
    drag = { pointerId: event.pointerId, session: beginDrag(event.clientX, event.timeStamp) };
    moving = true;
    button?.setPointerCapture(event.pointerId);
  }
  function pointerMove(event: PointerEvent) {
    if (drag?.pointerId !== event.pointerId) return;
    const distance = drag.session.move(event.clientX, event.timeStamp);
    offset = reduced() ? 0 : Math.max(-(button?.clientWidth ?? 0), Math.min(button?.clientWidth ?? 0, distance));
  }
  function pointerUp(event: PointerEvent) {
    if (drag?.pointerId !== event.pointerId) return;
    const { session } = drag;
    const distance = session.move(event.clientX, event.timeStamp);
    const passed = session.finish(button?.clientWidth ?? 0, distance < 0 ? -1 : 1, 0.35, event.timeStamp);
    // Arrasto que não chegou ao limiar também não vira toque (não abre o Tocando).
    swiped = passed || Math.abs(distance) > 10;
    drag = null;
    moving = false;
    offset = 0;
    if (passed) void (distance < 0 ? player.next() : player.previous());
  }
  function pointerCancel(event: PointerEvent) {
    if (drag?.pointerId !== event.pointerId) return;
    drag.session.cancel(); drag = null; moving = false; offset = 0;
  }
  function open() {
    if (swiped) { swiped = false; return; }
    navigate(paths.nowPlaying);
  }
</script>

{#if t}
  <div transition:springTransition class="relative border-t border-line bg-s2" role="region" aria-label="Mini player">
    <div class="absolute inset-x-0 top-0 h-0.5 bg-s3" aria-hidden="true"><div class="h-full" style="width:{player.progress * 100}%;background:var(--grad-lava)"></div></div>
    <div class="flex items-center gap-2 px-3 py-2">
      <button bind:this={button} class="flex min-h-12 min-w-0 flex-1 items-center gap-3 text-left touch-pan-y" style:transform={offset ? `translate3d(${offset}px,0,0)` : undefined} style:transition={moving || reduced() ? 'none' : 'transform var(--dur-base) var(--ease-spring)'}
        onpointerdown={pointerDown} onpointermove={pointerMove} onpointerup={pointerUp} onpointercancel={pointerCancel} onlostpointercapture={pointerCancel}
        onclick={open} aria-label="Abrir tocando agora: {t.title}">
        <Cover path={t.coverPath} thumb={t.thumbnailUrl} seed={t.id} size={40} rounded="0.375rem" />
        <span class="min-w-0">
          <span class="block truncate text-sm font-medium">{t.title}</span>
          <span class="block truncate text-xs text-muted">{t.artistName ?? 'Artista desconhecido'}{player.state.streaming ? ` · ${player.isLoading ? 'Conectando…' : 'Streaming'}` : ''}</span>
        </span>
      </button>
      <button class="icon-btn !h-12 !w-12" aria-label={player.isPlaying ? 'Pausar' : 'Tocar'} onclick={() => player.toggle()}>
        <Icon name={player.isPlaying ? 'pause' : 'play'} size={24} fill />
      </button>
      <button class="icon-btn !h-12 !w-12" aria-label="Próxima faixa" disabled={!player.hasNext} onclick={() => player.next()}><Icon name="next" size={22} /></button>
    </div>
  </div>
{/if}
