<script lang="ts">
  // @req RF-41 RNF-05 RNF-13
  import { onDestroy, type Snippet } from 'svelte';
  import { href, navigate } from '../stores/router.svelte';
  import MeshGradient from './MeshGradient.svelte';

  let { path, label, colors, class: cls = '', children, action }: {
    path: string; label: string; colors?: string[] | null; class?: string;
    children: Snippet; action?: Snippet<[() => void]>;
  } = $props();
  let highlighted = $state(false);
  // Timers separados: o destaque de uma ação interna não cancela a navegação pendente.
  let timer: ReturnType<typeof setTimeout> | undefined;
  let flash: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => { clearTimeout(timer); clearTimeout(flash); });

  function highlight() {
    highlighted = true;
    clearTimeout(flash);
    flash = setTimeout(() => { if (!timer) highlighted = false; }, 1500);
  }

  function open(event: MouseEvent | KeyboardEvent) {
    if (event instanceof MouseEvent && (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey)) return;
    event.preventDefault();
    if (matchMedia('(prefers-reduced-motion: reduce)').matches) { navigate(path); return; }
    highlighted = true;
    clearTimeout(timer);
    timer = setTimeout(() => navigate(path), 1500);
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === ' ') open(event);
  }
</script>

<MeshGradient class="dash-card rounded-xl border border-line {cls}" intensity={highlighted ? 0.65 : 0.22} colors={colors ?? undefined}>
  <div class="relative flex h-full flex-col">
    <a class="dash-card-link absolute inset-0 z-10 rounded-xl" href={href(path)} aria-label={label} onclick={open} onkeydown={keydown}></a>
    <div class="pointer-events-none flex-1">{@render children()}</div>
    {#if action}<div class="relative z-20 pointer-events-auto">{@render action(highlight)}</div>{/if}
  </div>
</MeshGradient>
