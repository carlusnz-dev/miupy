<script lang="ts">
  // @req RF-19
  import { onMount } from 'svelte';
  import { RecommendationsStore } from '../stores/recommendations.svelte';
  import { notify, notifyError } from '../stores/toast.svelte';
  import { fmtTime } from '../util/format';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';
  import State from './State.svelte';
  import SkeletonList from './SkeletonList.svelte';
  let { playlistId }: { playlistId?: string } = $props();
  // svelte-ignore state_referenced_locally
  const store = new RecommendationsStore(playlistId);
  let busy = $state<string | null>(null);
  onMount(() => { void store.start(); return () => store.stop(); });
  function reason(kind: string, label: string) {
    return kind === 'similarTrack' ? `Parecida com ${label}` : kind === 'artist' ? `Porque você ouve ${label}` : `Gênero ${label}`;
  }
  async function add(sourceId: string) {
    busy = sourceId;
    try { await store.add(sourceId); notify(playlistId ? 'Adicionada à playlist.' : 'Adicionada à discoteca.'); }
    catch (e) { notifyError(e); } finally { busy = null; }
  }
  async function dismiss(sourceId: string) {
    busy = sourceId;
    try { await store.dismiss(sourceId); } catch (e) { notifyError(e); } finally { busy = null; }
  }
</script>

<section class="mt-8" aria-labelledby="recommendations-heading">
  <div class="mb-3 flex items-center justify-between gap-3">
    <div><h2 id="recommendations-heading" class="text-lg font-semibold">{playlistId ? 'Sugestões para esta playlist' : 'Para você'}</h2><p class="text-xs text-muted">Descubra faixas a partir do que você ouve.</p></div>
    <button class="btn btn-sm" disabled={store.refreshing} onclick={() => store.refresh()}><Icon name="sparkle" size={15} />{store.refreshing ? 'Atualizando…' : 'Atualizar'}</button>
  </div>
  {#if store.error}<p class="mb-3 text-sm text-danger" role="alert">{store.error}</p>{/if}
  {#if !store.ready && !store.error}<SkeletonList variant="recs" label="Buscando sugestões" />
  {:else if !store.items.length}<State kind="empty" title="Ainda sem sugestões" text="As sugestões aparecem depois de ouvir música. Use Atualizar quando estiver conectado para buscar novas faixas." />
  {:else}
    <ul class="grid gap-2 sm:grid-cols-2">
      {#each store.items as r (r.sourceId)}
        <li class="card flex min-w-0 items-center gap-3 p-3 motion-item">
          <Cover url={r.thumbnailUrl} seed={r.sourceId} size={56} />
          <div class="min-w-0 flex-1"><p class="truncate font-medium">{r.title}</p><p class="truncate text-sm text-muted">{r.artist} · {fmtTime(r.durationMs)}</p><p class="truncate text-xs text-lava-text">{r.reasons.map((x) => reason(x.kind, x.label)).join(' · ')}</p></div>
          <div class="flex shrink-0 gap-1">
            <button class="icon-btn motion-bounce" aria-label={playlistId ? `Adicionar ${r.title} à playlist` : `Adicionar ${r.title} à discoteca`} disabled={busy === r.sourceId} onclick={() => add(r.sourceId)}><Icon name="plus" size={19} /></button>
            <button class="icon-btn" aria-label={`Dispensar ${r.title}`} disabled={busy === r.sourceId} onclick={() => dismiss(r.sourceId)}><Icon name="close" size={18} /></button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>
