<script lang="ts">
  // @req RF-47 RNF-08
  import { onMount, type Snippet } from 'svelte';
  import { spring } from '../motion';
  import { beginDrag } from '../util/drag';

  let { title, open = $bindable(false), children }: { title: string; open?: boolean; children: Snippet } = $props();
  let header: HTMLButtonElement;
  let sheet: HTMLElement;
  let drag: { pointerId: number; session: ReturnType<typeof beginDrag> } | null = null;
  let offset = $state(0);
  let moving = $state(false);
  let travel = 0;
  let suppressClick = false;
  const contentId = $props.id();
  const reduced = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const easing = `linear(${Array.from({ length: 25 }, (_, i) => spring(i / 24).toFixed(3)).join(',')})`;

  onMount(() => {
    const escape = (event: KeyboardEvent) => {
      if (!open || event.key !== 'Escape') return;
      event.preventDefault();
      event.stopImmediatePropagation();
      open = false;
      header.focus();
    };
    window.addEventListener('keydown', escape, true);
    return () => window.removeEventListener('keydown', escape, true);
  });

  function pointerDown(event: PointerEvent) {
    if (event.button !== 0) return;
    suppressClick = false;
    travel = sheet.clientHeight - header.clientHeight;
    drag = { pointerId: event.pointerId, session: beginDrag(event.clientY, event.timeStamp) };
    moving = true;
    header.setPointerCapture(event.pointerId);
  }
  function pointerMove(event: PointerEvent) {
    if (drag?.pointerId !== event.pointerId) return;
    const distance = drag.session.move(event.clientY, event.timeStamp);
    offset = reduced() ? 0 : open ? Math.max(0, Math.min(travel, distance)) : Math.max(-travel, Math.min(0, distance));
  }
  function pointerUp(event: PointerEvent) {
    if (drag?.pointerId !== event.pointerId) return;
    const distance = drag.session.move(event.clientY, event.timeStamp);
    const passed = drag.session.finish(travel, open ? 1 : -1, 0.3, event.timeStamp);
    suppressClick = Math.abs(distance) > 10;
    drag = null; moving = false; offset = 0;
    if (passed) { open = !open; header.focus(); }
  }
  function pointerCancel(event: PointerEvent) {
    if (drag?.pointerId !== event.pointerId) return;
    drag.session.cancel(); drag = null; moving = false; offset = 0;
  }
  function toggle() {
    if (suppressClick) { suppressClick = false; return; }
    open = !open;
  }
</script>

<section bind:this={sheet} class="fixed inset-x-0 bottom-0 z-10 flex h-[85dvh] flex-col glass-strong rounded-t-2xl border-x-0 border-b-0 pb-[var(--safe-b)]"
  style:transform={open ? `translate3d(0,${offset}px,0)` : `translate3d(0,calc(100% - 3.5rem - var(--safe-b) + ${offset}px),0)`}
  style:transition={moving || reduced() ? 'none' : `transform var(--dur-base) ${easing}`}
  aria-label="Folha: {title}">
  <div class="flex h-14 shrink-0 items-center gap-2 px-3">
    <button bind:this={header} type="button" class="flex h-14 min-w-0 flex-1 flex-col items-center justify-center touch-none rounded-lg focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-[var(--lava-text)]"
      aria-expanded={open} aria-controls={contentId} onclick={toggle}
      onpointerdown={pointerDown} onpointermove={pointerMove} onpointerup={pointerUp} onpointercancel={pointerCancel} onlostpointercapture={pointerCancel}>
      <span class="mb-1 h-1.5 w-10 rounded-full bg-s3" aria-hidden="true"></span>
      <span class="truncate text-sm font-semibold">{title}</span>
    </button>
    {#if open}<button type="button" class="btn !h-12" onclick={() => { open = false; header.focus(); }}>Fechar</button>{/if}
  </div>
  <div id={contentId} data-sheet-body class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-3 pb-3" inert={!open}>
    {@render children()}
  </div>
</section>
