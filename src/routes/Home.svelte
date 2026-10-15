<script lang="ts">
  // @req RF-19 RNF-11 RNF-13
  // Início: resumo da discoteca e atalhos. @req RF-14 RF-05
  import { api, type LibraryStats, type Track } from '../lib/ipc';
  import { mergeDeep, reconcileList } from '../lib/util/diff';
  import { Resource } from '../lib/stores/resource.svelte';
  import { library } from '../lib/stores/library.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { profile } from '../lib/stores/profile.svelte';
  import { href, paths } from '../lib/stores/router.svelte';
  import { fmtDuration, fmtNumber } from '../lib/util/format';
  import MeshGradient from '../lib/components/MeshGradient.svelte';
  import TrackRow from '../lib/components/TrackRow.svelte';
  import Cover from '../lib/components/Cover.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import State from '../lib/components/State.svelte';
  import Skeleton from '../lib/components/Skeleton.svelte';
  import SkeletonList from '../lib/components/SkeletonList.svelte';
  import Recommendations from '../lib/components/Recommendations.svelte';

  type HomeData = { stats: LibraryStats; recent: Track[] };
  // Cache entre telas: ao voltar para o Início, o último resumo aparece na hora e atualiza em
  // segundo plano; só o que mudou é trocado (RNF-11, RNF-13).
  const res = new Resource<HomeData>(['tracks', 'stats'], (prev, next) => {
    const stats = mergeDeep(prev?.stats, next.stats);
    const recent = reconcileList(prev?.recent, next.recent, (t) => t.id);
    return prev && prev.stats === stats && prev.recent === recent ? prev : { stats, recent };
  });
  res.prime('home');
  const stats = $derived(res.data?.stats ?? null);
  const recent = $derived(res.data?.recent ?? []);
  const loading = $derived(res.loading);
  const error = $derived(res.data === undefined ? res.error : null);
  $effect(() => res.attach());
  async function load() {
    await res.load('home', async () => {
      const [stats, recent] = await Promise.all([
        api.statsLibrary(),
        api.libraryListTracks({ query: { sortBy: 'addedAt', sortDir: 'desc' } }).then((l) => l.slice(0, 6)),
      ]);
      return { stats, recent };
    });
  }
  $effect(() => { void load(); });

  const hour = new Date().getHours();
  const greeting = hour < 12 ? 'Bom dia' : hour < 18 ? 'Boa tarde' : 'Boa noite';
  const shortcuts = [
    { label: 'Discoteca', icon: 'library', path: paths.library, text: 'Todas as faixas' },
    { label: 'Playlists', icon: 'playlist', path: paths.playlists, text: 'Suas listas' },
    { label: 'Buscar no YouTube', icon: 'search', path: paths.youtube, text: 'Achar e baixar' },
    { label: 'Retrospectiva', icon: 'sparkle', path: paths.wrapped, text: 'Seu ano em música' },
  ];
</script>

<MeshGradient class="mb-6 rounded-2xl border border-line">
  <div class="p-5 md:p-8">
    <p class="eyebrow mb-1 !text-fg/70">{greeting}{profile.data ? `, ${profile.data.displayName.split(' ')[0]}` : ''}</p>
    <h1 class="text-3xl font-bold tracking-tight md:text-4xl">Sua discoteca</h1>
    {#if stats}
      <dl class="mt-5 grid grid-cols-2 gap-x-6 gap-y-3 md:grid-cols-4">
        {#each [['Faixas', fmtNumber(stats.trackCount)], ['Baixadas', fmtNumber(stats.downloadedCount)], ['Playlists', fmtNumber(stats.playlistCount)], ['Artistas', fmtNumber(stats.artistCount)]] as [k, v]}
          <div><dt class="text-xs text-fg/70">{k}</dt><dd class="tnum text-2xl font-bold">{v}</dd></div>
        {/each}
      </dl>
      <p class="mt-4 text-sm text-fg/80">{fmtDuration(stats.totalDurationMs)} de música, {fmtDuration(stats.totalListenedMs)} ouvidos no total.</p>
    {:else if loading}
      <div class="mt-5 grid grid-cols-2 gap-x-6 gap-y-3 md:grid-cols-4" aria-hidden="true">
        {#each [0, 1, 2, 3] as i (i)}<div><Skeleton w="3.5rem" h="0.65rem" class="mb-2" /><Skeleton w="4.5rem" h="1.5rem" /></div>{/each}
      </div>
      <Skeleton w="min(18rem, 80%)" h="0.8rem" class="mt-4" />
    {/if}
    {#if player.active}
      <a class="btn btn-primary mt-5" href={href(paths.nowPlaying)}><Icon name="play" size={16} fill />Voltar ao que está tocando</a>
    {:else if recent.length}
      <button class="btn btn-primary mt-5" onclick={() => player.play(recent[0].id, recent.map((t) => t.id))}><Icon name="play" size={16} fill />Tocar as mais recentes</button>
    {/if}
  </div>
</MeshGradient>

<section class="mb-8 grid grid-cols-2 gap-3 md:grid-cols-4" aria-label="Atalhos">
  {#each shortcuts as s}
    <a href={href(s.path)} class="card flex flex-col gap-3 p-4 hover:border-line-strong hover:bg-s2">
      <span class="grid h-9 w-9 place-items-center rounded-lg text-lava-ink" style="background:var(--grad-lava)"><Icon name={s.icon} size={18} /></span>
      <span><span class="block font-semibold">{s.label}</span><span class="text-xs text-muted">{s.text}</span></span>
    </a>
  {/each}
</section>

{#if loading}
  <div class="grid gap-8 lg:grid-cols-2">
    <section aria-label="Adicionadas recentemente"><Skeleton w="12rem" h="1.1rem" class="mb-3" /><SkeletonList variant="tracks" count={6} label="Carregando faixas recentes" /></section>
    <section aria-label="Mais tocadas"><Skeleton w="8rem" h="1.1rem" class="mb-3" /><SkeletonList variant="tracks" count={6} label="Carregando as mais tocadas" /></section>
  </div>
{:else if error}
  <State kind="error" text={error} onretry={load} />
{:else if stats && stats.trackCount === 0}
  <State kind="empty" title="Sua discoteca está vazia" text="Busque músicas no YouTube e baixe para ouvir offline.">
    <a class="btn btn-primary" href={href(paths.youtube)}>Buscar no YouTube</a>
  </State>
{:else}
  <div class="grid gap-8 lg:grid-cols-2">
    <section aria-labelledby="h-recent">
      <h2 id="h-recent" class="mb-2 text-lg font-semibold">Adicionadas recentemente</h2>
      <div role="list">
        {#each recent as t, i (t.id)}
          <TrackRow track={t} index={i} current={player.track?.id === t.id} playing={player.isPlaying} onplay={() => player.play(t.id, recent.map((x) => x.id))} />
        {/each}
      </div>
    </section>
    <section aria-labelledby="h-top">
      <h2 id="h-top" class="mb-2 text-lg font-semibold">Mais tocadas</h2>
      {#if stats && stats.topTracks.length}
        <div role="list">
          {#each stats.topTracks as r, i (r.track.id)}
            <TrackRow track={r.track} index={i} current={player.track?.id === r.track.id} playing={player.isPlaying} onplay={() => player.play(r.track.id, stats!.topTracks.map((x) => x.track.id))}>
              {#snippet extra()}<span class="tnum hidden w-16 text-right text-xs text-faint sm:block">{r.plays}×</span>{/snippet}
            </TrackRow>
          {/each}
        </div>
      {:else}
        <State kind="empty" title="Ainda sem reproduções" text="As mais tocadas aparecem aqui depois que você ouvir algumas faixas." />
      {/if}
    </section>
    {#if library.playlists.length}
      <section class="lg:col-span-2" aria-labelledby="h-pl">
        <div class="mb-2 flex items-baseline justify-between"><h2 id="h-pl" class="text-lg font-semibold">Playlists</h2><a class="text-sm text-lava-text hover:underline" href={href(paths.playlists)}>Ver todas</a></div>
        <ul class="grid grid-cols-2 gap-3 md:grid-cols-4">
          {#each library.playlists.slice(0, 4) as p (p.id)}
            <li><a href={href(paths.playlist(p.id))} class="card block p-3 hover:bg-s2">
              <Cover path={p.coverPath} seed={p.id} class="mb-3 w-full" />
              <span class="block truncate font-medium">{p.name}</span><span class="text-xs text-muted">{p.trackCount} faixas</span>
            </a></li>
          {/each}
        </ul>
      </section>
    {/if}
  </div>
{/if}

<Recommendations />
