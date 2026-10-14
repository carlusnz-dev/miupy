<script lang="ts">
  import { toasts } from '../stores/toast.svelte';
</script>

<div class="pointer-events-none fixed inset-x-0 z-50 flex flex-col items-center gap-2 px-4" style="bottom: calc(var(--player-h) + var(--nav-h) + var(--safe-b) + 0.75rem)" aria-live="polite">
  {#each toasts.items as t (t.id)}
    <div class="pointer-events-auto max-w-md rounded-lg border bg-s3 px-4 py-2.5 text-sm {t.kind === 'error' ? 'border-danger/50 text-fg' : 'border-line-strong'}" role={t.kind === 'error' ? 'alert' : 'status'}>
      <span class={t.kind === 'error' ? 'text-danger' : t.kind === 'success' ? 'text-lava-text' : ''}>●</span>
      {t.text}
    </div>
  {/each}
</div>
