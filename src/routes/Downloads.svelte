<script lang="ts">
  // Fila de downloads com progresso e cancelamento. @req RF-10 RF-11 RNF-11 RNF-13
  import { api, type DownloadJob, type Track } from '../lib/ipc';
  import { downloads } from '../lib/stores/downloads.svelte';
  import { Resource } from '../lib/stores/resource.svelte';
  import { mergeList } from '../lib/util/diff';
  import { notifyError } from '../lib/stores/toast.svelte';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import Cover from '../lib/components/Cover.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import State from '../lib/components/State.svelte';
  import Skeleton from '../lib/components/Skeleton.svelte';
  import SkeletonList from '../lib/components/SkeletonList.svelte';

  // Títulos e capas dos jobs: lista em cache, atualizada em segundo plano e só quando as faixas mudam.
  const names = new Resource<Track[]>(['tracks'], mergeList((t: Track) => t.id));
  names.prime('downloads:titles');
  const titles = $derived(new Map((names.data ?? []).map((t) => [t.id, t])));
  const titlesReady = $derived(names.data !== undefined);
  $effect(() => names.attach());
  $effect(() => { void names.load('downloads:titles', () => api.libraryListTracks()); });

  const active = $derived(downloads.jobs.filter((j) => j.status === 'queued' || j.status === 'downloading'));
  // @req RF-31
  const waiting = $derived(downloads.jobs.filter((j) => j.status === 'error' && j.error?.startsWith('auto_offline:')));
  const finished = $derived(downloads.jobs.filter((j) => j.status === 'done' || (j.status === 'error' && !j.error?.startsWith('auto_offline:'))).reverse());

  async function cancel(j: DownloadJob) {
    try { await api.downloadCancel({ jobId: j.id }); } catch (e) { notifyError(e); }
  }
  async function clearFinished() {
    try { await downloads.clearFinished(); } catch (e) { notifyError(e); }
  }
  const label: Record<string, string> = { queued: 'Na fila', downloading: 'Baixando', done: 'Concluído', error: 'Falhou', none: '' };
</script>

{#snippet item(j: DownloadJob)}
  {@const t = titles.get(j.trackId)}
  <li class="card flex items-center gap-3 p-3">
    <Cover path={t?.coverPath} seed={j.trackId} size={44} rounded="0.375rem" />
    <div class="min-w-0 flex-1">
      {#if t || titlesReady}
        <p class="truncate text-sm font-medium">{t?.title ?? j.trackId}</p>
      {:else}
        <Skeleton w="45%" h="0.8rem" class="my-0.5" />
      {/if}
      <p class="truncate text-xs {j.status === 'error' && !j.error?.startsWith('auto_offline:') ? 'text-danger' : 'text-muted'}">
        {t?.artistName ? `${t.artistName} · ` : ''}{j.error?.startsWith('auto_offline:') ? 'Aguardando rede' : j.status === 'error' && j.error ? j.error : label[j.status]}
      </p>
      {#if j.status === 'downloading' || j.status === 'queued'}
        <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-s3" role="progressbar" aria-label="Progresso do download de {t?.title ?? 'faixa'}" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(j.progress * 100)}>
          <div class="h-full w-full origin-left rounded-full transition-transform duration-300" style="transform:scaleX({Math.max(0.02, j.progress)});background:var(--grad-lava)"></div>
        </div>
      {/if}
    </div>
    {#if j.status === 'downloading' || j.status === 'queued'}
      <span class="tnum w-10 text-right text-xs text-muted">{Math.round(j.progress * 100)}%</span>
      <button class="icon-btn" aria-label="Cancelar download de {t?.title ?? 'faixa'}" onclick={() => cancel(j)}><Icon name="close" size={18} /></button>
    {:else if j.status === 'done'}
      <span class="text-ok" role="img" aria-label="Concluído"><Icon name="check" size={20} /></span>
    {:else}
      <span class="{j.error?.startsWith('auto_offline:') ? 'text-muted' : 'text-danger'}" role="img" aria-label={j.error?.startsWith('auto_offline:') ? 'Aguardando rede' : 'Falhou'}><Icon name="alert" size={20} /></span>
    {/if}
  </li>
{/snippet}

<PageHeader title="Downloads" subtitle={active.length ? `${active.length} em andamento` : 'Nada em andamento'} />

{#if downloads.loading}
  <SkeletonList variant="jobs" count={4} label="Carregando os downloads" />
{:else if downloads.error}
  <State kind="error" text={downloads.error} onretry={() => downloads.reload()} />
{:else if downloads.jobs.length === 0}
  <State kind="empty" icon="download" title="Nenhum download" text="Baixe faixas pela discoteca, por uma playlist ou pela busca do YouTube." />
{:else}
  {#if active.length}
    <h2 class="eyebrow mb-2">Em andamento</h2>
    <ul class="mb-6 flex flex-col gap-2">{#each active as j (j.id)}{@render item(j)}{/each}</ul>
  {/if}
  {#if waiting.length}
    <h2 class="eyebrow mb-2">Aguardando rede</h2>
    <ul class="mb-6 flex flex-col gap-2">{#each waiting as j (j.id)}{@render item(j)}{/each}</ul>
  {/if}
  {#if finished.length}
    <div class="mb-2 flex items-center justify-between"><h2 class="eyebrow">Finalizados</h2><button class="btn btn-sm" onclick={clearFinished}>Limpar concluídos</button></div>
    <ul class="flex flex-col gap-2">{#each finished as j (j.id)}{@render item(j)}{/each}</ul>
  {/if}
{/if}
