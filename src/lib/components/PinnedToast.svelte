<script lang="ts">
  // @req RF-32
  import type { Snippet } from 'svelte';
  import { springTransition } from '../motion';

  let { title, content, onclose }: { title: string; content: Snippet; onclose: () => void } = $props();
</script>

<aside class="pinned-toast fixed right-4 z-40 w-[min(24rem,calc(100vw-2rem))] rounded-xl border border-line p-4 text-sm text-fg" style="bottom: calc(var(--player-h) + var(--nav-h) + var(--safe-b) + 0.75rem)" transition:springTransition={{ y: 8, duration: 220 }} role="status" aria-label={title}>
  <div class="flex items-start justify-between gap-3">
    <strong class="font-semibold">{title}</strong>
    <button type="button" class="icon-btn -mr-2 -mt-2" aria-label="Fechar" onclick={onclose}>×</button>
  </div>
  {@render content()}
</aside>

<style>
  .pinned-toast { background: var(--surface-2); }
  @supports ((-webkit-backdrop-filter: blur(1px)) or (backdrop-filter: blur(1px))) {
    .pinned-toast {
      background: color-mix(in srgb, var(--surface-2) 88%, transparent);
      -webkit-backdrop-filter: blur(8px);
      backdrop-filter: blur(8px);
    }
  }
</style>
