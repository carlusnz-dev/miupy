<script lang="ts">
  // @req RF-32
  import { api, type MetadataRefreshStatus } from '../../ipc';
  import { notifyError } from '../../stores/toast.svelte';

  let metadata = $state<MetadataRefreshStatus | null>(null);
  let onlyMissing = $state(true);
  let refreshing = $state(false);

  $effect(() => {
    let alive = true;
    let un: (() => void) | undefined;
    let revision = 0;
    void api.on('metadata://progress', (status) => {
      if (alive) { revision++; metadata = status; }
    }).then(async (unsubscribe) => {
      if (!alive) { unsubscribe(); return; }
      un = unsubscribe;
      const before = revision;
      try {
        const status = await api.metadataRefreshStatus();
        if (alive && before === revision) metadata = status;
      } catch (e) { if (alive) notifyError(e); }
    }).catch((e) => { if (alive) notifyError(e); });
    return () => { alive = false; un?.(); };
  });

  async function refreshMetadata() {
    refreshing = true;
    try {
      await api.metadataRefreshLibrary({ onlyMissing });
      metadata = await api.metadataRefreshStatus();
    } catch (e) { notifyError(e); }
    finally { refreshing = false; }
  }

  async function stopMetadata() {
    try { await api.metadataRefreshCancel(); }
    catch (e) { notifyError(e); }
  }
</script>

<section class="mb-8 card p-5" aria-labelledby="h-metadata">
  <h2 id="h-metadata" class="text-lg font-semibold">Metadados</h2>
  <label class="my-3 flex items-center gap-2 text-sm"><input type="checkbox" bind:checked={onlyMissing} />Reprocessar apenas dados ausentes</label>
  <button class="btn" disabled={refreshing || metadata?.running} onclick={refreshMetadata}>Reprocessar metadados</button>
  {#if metadata?.running}
    <div class="mt-3 flex items-center gap-3">
      <p class="text-sm text-muted" role="status">{metadata.done} de {metadata.total} faixas</p>
      <button type="button" class="btn btn-sm" disabled={metadata.stopping} onclick={stopMetadata}>{metadata.stopping ? 'Parando…' : 'Parar'}</button>
    </div>
    <div class="mt-2 h-2 w-full max-w-sm overflow-hidden rounded-full bg-s2" role="progressbar" aria-label="Andamento do reprocessamento de metadados" aria-valuemin="0" aria-valuemax={Math.max(metadata.total, 1)} aria-valuenow={metadata.done}>
      <div class="h-full bg-lava" style:width={`${metadata.total ? metadata.done / metadata.total * 100 : 0}%`}></div>
    </div>
  {:else if metadata?.cancelled}
    <p class="mt-3 text-sm text-muted" role="status">Parado em {metadata.done} de {metadata.total} faixas · {metadata.failed} com falha</p>
  {:else if metadata?.finishedAt != null && metadata.total === 0}
    <p class="mt-3 text-sm text-muted" role="status">Nenhuma faixa para reprocessar.</p>
  {:else if metadata?.finishedAt !== null && metadata?.finishedAt !== undefined}
    <p class="mt-3 text-sm text-muted" role="status">
      {metadata.done - metadata.failed} com sucesso · {metadata.failed} com falha · Concluído em {new Intl.DateTimeFormat('pt-BR', { dateStyle: 'short', timeStyle: 'short' }).format(metadata.finishedAt)}
    </p>
  {/if}
</section>
