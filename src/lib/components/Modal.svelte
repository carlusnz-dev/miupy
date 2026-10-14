<script lang="ts">
  // Diálogo nativo <dialog>: já traz trap de foco, Esc e backdrop.
  import Icon from './Icon.svelte';
  type Props = { open: boolean; title: string; onclose: () => void; children: import('svelte').Snippet; footer?: import('svelte').Snippet };
  let { open, title, onclose, children, footer }: Props = $props();
  let d: HTMLDialogElement;
  $effect(() => {
    if (open && !d.open) d.showModal();
    else if (!open && d.open) d.close();
  });
</script>

<dialog bind:this={d} class="modal" aria-label={title} onclose={onclose} onclick={(e) => { if (e.target === d) onclose(); }}>
  {#if open}
    <div class="flex max-h-[inherit] flex-col">
      <div class="flex items-center justify-between border-b border-line px-4 py-3">
        <h2 class="text-base font-semibold">{title}</h2>
        <button class="icon-btn" aria-label="Fechar" onclick={onclose}><Icon name="close" size={18} /></button>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto p-4">{@render children()}</div>
      {#if footer}<div class="flex justify-end gap-2 border-t border-line px-4 py-3">{@render footer()}</div>{/if}
    </div>
  {/if}
</dialog>
