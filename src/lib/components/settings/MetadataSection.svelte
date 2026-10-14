<script lang="ts">
  // Reprocessamento de metadados em lote. @req RF-12
  import { api } from '../../ipc';
  import { notifyError } from '../../stores/toast.svelte';

  let metadata = $state<{ done: number; total: number } | null>(null);
  let onlyMissing = $state(true);
  let refreshing = $state(false);
  $effect(() => {
    let alive = true; let un: (() => void) | undefined;
    void api.on('metadata://progress', (p) => { if (alive) metadata = p; }).then((u) => { if (alive) un = u; else u(); });
    return () => { alive = false; un?.(); };
  });
  async function refreshMetadata() { refreshing = true; metadata = null; try { const total = await api.metadataRefreshLibrary({ onlyMissing }); if (total === 0) metadata = { done: 0, total: 0 }; } catch (e) { notifyError(e); } finally { refreshing = false; } }
</script>

<section class="mb-8 card p-5" aria-labelledby="h-metadata">
  <h2 id="h-metadata" class="text-lg font-semibold">Metadados</h2>
  <label class="my-3 flex items-center gap-2 text-sm"><input type="checkbox" bind:checked={onlyMissing} />Reprocessar apenas dados ausentes</label>
  <button class="btn" disabled={refreshing} onclick={refreshMetadata}>Reprocessar metadados</button>
  {#if metadata}<p class="mt-2 text-sm text-muted" role="status">{metadata.total ? `${metadata.done} de ${metadata.total} faixas` : 'Nenhuma faixa para reprocessar.'}</p>{/if}
</section>
