<script lang="ts">
  // Estatísticas da discoteca. @req RF-14
  import { api, errorMessage, type LibraryStats } from '../lib/ipc';
  import { library } from '../lib/stores/library.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { href, paths } from '../lib/stores/router.svelte';
  import { fmtDuration, fmtNumber } from '../lib/util/format';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import TrackRow from '../lib/components/TrackRow.svelte';
  import State from '../lib/components/State.svelte';
  import Icon from '../lib/components/Icon.svelte';

  let s = $state<LibraryStats | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  async function load() {
    try { s = await api.statsLibrary(); error = null; } catch (e) { error = errorMessage(e); } finally { loading = false; }
  }
  $effect(() => { library.version; void load(); });

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
  <State kind="loading" />
{:else if error}
  <State kind="error" text={error} onretry={load} />
{:else if s}
  <dl class="mb-8 grid grid-cols-2 gap-3 md:grid-cols-3">
    {#each cards as c}
      <div class="card p-4">
        <dt class="text-xs text-muted">{c.k}</dt>
        <dd class="tnum mt-1 text-3xl font-bold text-grad">{c.v}</dd>
        {#if c.hint}<dd class="mt-0.5 text-xs text-faint">{c.hint}</dd>{/if}
      </div>
    {/each}
  </dl>

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
