<script lang="ts">
  // @req RNF-08
  import { springTransition } from '../motion';
  import { toasts } from '../stores/toast.svelte';
  import Icon from './Icon.svelte';
</script>

<div class="pointer-events-none fixed inset-x-0 z-50 flex flex-col items-center gap-2 px-4" style="bottom: calc(var(--player-h) + var(--nav-h) + var(--safe-b) + 0.75rem)" aria-live="polite">
  {#each toasts.items as t (t.id)}
    <!-- @req RNF-04 -->
    <div transition:springTransition={{ y: 8, duration: 220 }} class="toast-glass pointer-events-auto flex w-full max-w-md items-start gap-3 rounded-xl px-5 py-4 text-sm text-fg" role={t.kind === 'error' ? 'alert' : 'status'}>
      <span class="mt-0.5 shrink-0 {t.kind === 'error' ? 'text-danger' : t.kind === 'success' ? 'text-ok' : 'text-lava-text'}">
        <Icon name={t.kind === 'error' ? 'alert' : t.kind === 'success' ? 'check' : 'sparkle'} size={18} />
      </span>
      <span>{t.text}</span>
    </div>
  {/each}
</div>
