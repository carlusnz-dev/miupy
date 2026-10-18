<script lang="ts">
  // Diálogo nativo <dialog>: já traz trap de foco, Esc e backdrop. Abre centralizado com pop-up. @req RNF-10
  import Icon from './Icon.svelte';
  type Props = { open: boolean; title: string; onclose: () => void; children: import('svelte').Snippet; footer?: import('svelte').Snippet };
  let { open, title, onclose, children, footer }: Props = $props();
  let d = $state<HTMLDialogElement>();
  let closing = $state(false);
  let closeTimer: ReturnType<typeof setTimeout> | undefined;
  let internalClose = false;
  function closeAnimated() {
    if (!d?.open || closing) return;
    closing = true;
    closeTimer = setTimeout(() => {
      internalClose = true;
      d?.close();
      closing = false;
      onclose();
    }, matchMedia('(prefers-reduced-motion: reduce)').matches ? 90 : 190);
  }
  $effect(() => {
    if (open && d && !d.open) { clearTimeout(closeTimer); closing = false; d.showModal(); }
    else if (!open && d?.open) closeAnimated();
  });
</script>

<dialog bind:this={d} class="modal glass-strong" class:closing aria-label={title} oncancel={(e) => { e.preventDefault(); closeAnimated(); }} onclose={() => { if (!internalClose) onclose(); internalClose = false; }} onclick={(e) => { if (e.target === d) closeAnimated(); }}>
  {#if open || d?.open}
    <div class="flex max-h-[inherit] flex-col">
      <div class="flex items-center justify-between border-b border-line px-4 py-3">
        <h2 class="text-base font-semibold">{title}</h2>
        <button class="icon-btn" aria-label="Fechar" onclick={closeAnimated}><Icon name="close" size={18} /></button>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto p-4">{@render children()}</div>
      {#if footer}<div class="flex justify-end gap-2 border-t border-line px-4 py-3">{@render footer()}</div>{/if}
    </div>
  {/if}
</dialog>
