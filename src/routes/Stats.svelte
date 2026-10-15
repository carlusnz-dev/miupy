<script lang="ts">
  // @req RF-19 RNF-11 RNF-13
  // Estatísticas da discoteca. @req RF-14
  import { api, type LibraryStats, type TasteProfile } from '../lib/ipc';
  import { mergeDeep } from '../lib/util/diff';
  import { Resource } from '../lib/stores/resource.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { href, paths } from '../lib/stores/router.svelte';
  import { fmtDuration, fmtNumber } from '../lib/util/format';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import TrackRow from '../lib/components/TrackRow.svelte';
  import State from '../lib/components/State.svelte';
  import SkeletonList from '../lib/components/SkeletonList.svelte';
  import Icon from '../lib/components/Icon.svelte';

  type StatsData = { s: LibraryStats; taste: TasteProfile };
  const res = new Resource<StatsData>(['tracks', 'stats'], (prev, next) => {
    const s = mergeDeep(prev?.s, next.s), taste = mergeDeep(prev?.taste, next.taste);
    return prev && prev.s === s && prev.taste === taste ? prev : { s, taste };
  });
  res.prime('stats');
  const s = $derived(res.data?.s ?? null);
  const taste = $derived(res.data?.taste ?? null);
  const loading = $derived(res.loading);
  const error = $derived(res.data === undefined ? res.error : null);
  $effect(() => res.attach());
  async function load() {
    await res.load('stats', async () => {
      const [s, taste] = await Promise.all([api.statsLibrary(), api.statsTaste()]);
      return { s, taste };
    });
  }
  $effect(() => { void load(); });

  const cards = $derived(s ? [
    { k: 'Faixas', v: fmtNumber(s.trackCount), hint: `${fmtNumber(s.downloadedCount)} baixadas` },
    { k: 'Playlists', v: fmtNumber(s.playlistCount), hint: '' },
    { k: 'Artistas', v: fmtNumber(s.artistCount), hint: '' },
    { k: 'Favoritas', v: fmtNumber(s.favoriteCount), hint: '' },
    { k: 'Reproduções', v: fmtNumber(s.totalPlays), hint: '' },
    { k: 'Tempo ouvido', v: fmtDuration(s.totalListenedMs), hint: `de ${fmtDuration(s.totalDurationMs)} de acervo` },
  ] : []);
  const maxPlays = $derived(Math.max(1, ...(s?.topTracks.map((t) => t.plays) ?? [1])));
</script>

<PageHeader title="Estatísticas" eyebrow="Discoteca">
  {#snippet actions()}<a class="btn" href={href(paths.wrapped)}><Icon name="sparkle" size={16} />Retrospectiva anual</a>{/snippet}
</PageHeader>

{#if loading}
  <SkeletonList variant="stats" class="mb-8" label="Carregando as estatísticas" />
  <SkeletonList variant="tracks" count={6} />
{:else if error}
  <State kind="error" text={error} onretry={load} />
{:else if s}
  <dl class="mb-8 grid grid-cols-2 gap-3 md:grid-cols-3">
    {#each cards as c}
      <div class="card p-4">
        <dt class="text-xs text-muted">{c.k}</dt>
        <dd class="tnum font-display mt-1 text-3xl font-bold text-grad">{c.v}</dd>
        {#if c.hint}<dd class="mt-0.5 text-xs text-faint">{c.hint}</dd>{/if}
      </div>
    {/each}
  </dl>

  {#if taste}
    <section class="card mb-8 p-4" aria-labelledby="taste-heading">
      <h2 id="taste-heading" class="mb-2 text-lg font-semibold">Seu gosto musical</h2>
      <p class="text-sm text-muted">{fmtDuration(taste.totalListenedMs)} ouvidos · {Math.round(taste.skipRate * 100)}% de faixas puladas</p>
      <div class="mt-3 grid gap-4 sm:grid-cols-2">
        <div><h3 class="mb-1 text-sm font-semibold">Artistas</h3>{#each taste.topArtists as a}<p class="truncate text-sm text-muted">{a.name}</p>{/each}</div>
        <div><h3 class="mb-1 text-sm font-semibold">Gêneros</h3>{#each taste.topGenres as g}<span class="chip mr-1 mb-1">{g.genre}</span>{/each}</div>
      </div>
    </section>
  {/if}
  <h2 class="mb-2 text-lg font-semibold">Mais tocadas</h2>
  {#if s.topTracks.length === 0}
    <State kind="empty" title="Sem reproduções ainda" text="Ouça algumas faixas e elas aparecem aqui." />
  {:else}
    <div role="list">
      {#each s.topTracks as r, i (r.track.id)}
        <div class="relative">
          <div class="absolute inset-y-1 left-0 -z-0 rounded-lg opacity-20" style="width:{(r.plays / maxPlays) * 100}%;background:var(--grad-lava)" aria-hidden="true"></div>
          <TrackRow track={r.track} index={i} current={player.track?.id === r.track.id} playing={player.isPlaying} onplay={() => player.play(r.track.id, s!.topTracks.map((x) => x.track.id))}>
            {#snippet extra()}<span class="tnum w-24 text-right text-xs text-muted">{r.plays} {r.plays === 1 ? 'vez' : 'vezes'}</span>{/snippet}
          </TrackRow>
        </div>
      {/each}
    </div>
  {/if}
{/if}
