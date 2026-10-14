<script lang="ts">
  // @req RF-25 RNF-04
  import { onMount } from 'svelte';
  import { dna } from '../lib/stores/dna.svelte';
  import { fmtDuration } from '../lib/util/format';
  import { fmtScore } from '../lib/stores/scores.svelte';
  import { href, paths } from '../lib/stores/router.svelte';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import State from '../lib/components/State.svelte';
  onMount(() => { void dna.load(); });
  const pct = (v: number) => `${Math.round(v * 100)}%`;
</script>
<PageHeader title="DNA" subtitle="O que você mais ouviu e como seu gosto mudou." />
<label class="mb-5 flex items-center gap-3 text-sm">Período
  <select class="field !w-auto" value={dna.windowDays} onchange={(e) => dna.load(Number(e.currentTarget.value))}>
    <option value="7">7 dias</option><option value="30">30 dias</option><option value="90">90 dias</option><option value="365">365 dias</option>
  </select>
</label>
{#if dna.loading && !dna.data}<State kind="loading" />
{:else if dna.error}<State kind="error" text={dna.error} onretry={() => dna.load()} />
{:else if !dna.data || (dna.data.listenedMs === 0 && dna.data.topArtists.length === 0 && dna.data.topGenres.length === 0)}<State kind="empty" title="Seu DNA ainda está se formando" text="Ouça algumas faixas para ver seus artistas, gêneros e tendências." />
{:else}
  <div class="mb-5 grid gap-3 sm:grid-cols-3">
    <div class="card p-5"><p class="text-sm text-muted">Tempo ouvido</p><p class="mt-1 text-2xl font-bold text-lava-text">{fmtDuration(dna.data.listenedMs)}</p><p class="text-xs text-faint">Antes: {fmtDuration(dna.data.previousListenedMs)}</p></div>
    <div class="card p-5"><p class="text-sm text-muted">Taxa de pulos</p><p class="mt-1 text-2xl font-bold text-lava-text">{pct(dna.data.skipRate)}</p><p class="text-xs text-faint">Antes: {pct(dna.data.previousSkipRate)}</p></div>
    <div class="card p-5"><p class="text-sm text-muted">Variação de pulos</p><p class="mt-1 text-2xl font-bold text-lava-text">{Math.round((dna.data.skipRate - dna.data.previousSkipRate) * 100)} pontos</p></div>
  </div>
  <div class="grid gap-5 md:grid-cols-2">
    <section class="card p-5"><h2 class="mb-3 text-lg font-semibold">Top artistas</h2>{#each dna.data.topArtists as a}<p class="flex justify-between gap-2 py-1 text-sm"><a href={href(paths.artist(a.artistId))} class="hover:underline">{a.name}</a><span class="tnum text-lava-text">{fmtScore(a.score)}</span></p>{/each}</section>
    <section class="card p-5"><h2 class="mb-3 text-lg font-semibold">Top gêneros</h2>{#each dna.data.topGenres as g}<p class="flex justify-between gap-2 py-1 text-sm"><span>{g.genre}</span><span class="tnum text-lava-text">{fmtScore(g.score)}</span></p>{/each}</section>
    <section class="card p-5"><h2 class="mb-3 text-lg font-semibold">Subiu</h2>{#each dna.data.rising as t}<p class="flex justify-between gap-2 py-1 text-sm"><span>{t.label}</span><span class="tnum text-ok">+{fmtScore(t.delta)}</span></p>{:else}<p class="text-sm text-muted">Sem alta neste período.</p>{/each}</section>
    <section class="card p-5"><h2 class="mb-3 text-lg font-semibold">Caiu</h2>{#each dna.data.falling as t}<p class="flex justify-between gap-2 py-1 text-sm"><span>{t.label}</span><span class="tnum text-muted">{fmtScore(t.delta)}</span></p>{:else}<p class="text-sm text-muted">Sem queda neste período.</p>{/each}</section>
  </div>
{/if}
