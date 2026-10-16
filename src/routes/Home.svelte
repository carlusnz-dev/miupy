<script lang="ts">
  // @req RF-14 RF-19 RF-41 RF-42 RNF-11 RNF-13
  import { api, type LibraryStats, type Track } from '../lib/ipc';
  import { mergeDeep, reconcileList } from '../lib/util/diff';
  import { Resource } from '../lib/stores/resource.svelte';
  import { library } from '../lib/stores/library.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { profile } from '../lib/stores/profile.svelte';
  import { paths } from '../lib/stores/router.svelte';
  import { fmtDuration, fmtNumber } from '../lib/util/format';
  import { coverPalette } from '../lib/util/palette';
  import { assetCrossOrigin } from '../lib/util/asset';
  import { enterStyle } from '../lib/motion';
  import DashCard from '../lib/components/DashCard.svelte';
  import PlaylistCover from '../lib/components/PlaylistCover.svelte';
  import Cover from '../lib/components/Cover.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import State from '../lib/components/State.svelte';
  import Skeleton from '../lib/components/Skeleton.svelte';
  import Recommendations from '../lib/components/Recommendations.svelte';

  type HomeData = { stats: LibraryStats; recent: Track[] };
  const res = new Resource<HomeData>(['tracks', 'stats'], (prev, next) => {
    const stats = mergeDeep(prev?.stats, next.stats);
    const recent = reconcileList(prev?.recent, next.recent, (t) => t.id);
    return prev && prev.stats === stats && prev.recent === recent ? prev : { stats, recent };
  });
  res.prime('home');
  const stats = $derived(res.data?.stats ?? null);
  const topTracks = $derived(stats?.topTracks.slice(0, 3) ?? []);
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
  const currentPlaylist = $derived(player.state.context?.kind === 'playlist' ? library.playlists.find((p) => p.id === player.state.context?.id) : undefined);
  const currentCover = $derived(currentPlaylist?.coverPath ?? currentPlaylist?.mosaicPaths[0] ?? player.track?.coverPath ?? null);
  const coverSrc = $derived(api.resolveAsset(currentCover) ?? player.track?.thumbnailUrl ?? null);
  let palette = $state<string[] | null>(null);
  $effect(() => {
    const src = coverSrc;
    palette = null;
    if (!src) return;
    let active = true;
    void coverPalette(src, assetCrossOrigin(src)).then((colors) => {
      if (active) palette = colors?.length ? [colors[0], colors[1] ?? colors[0], colors[2] ?? colors[1] ?? colors[0]] : null;
    });
    return () => { active = false; };
  });
  const contextName = $derived(player.state.context?.label || (player.track ? 'Sua fila' : 'Suas músicas recentes'));
  const contextPath = $derived(player.track ? paths.nowPlaying : paths.library);
  const shortcuts = [
    { label: 'Discoteca', icon: 'library', path: paths.library, text: 'Todas as faixas' },
    { label: 'Playlists', icon: 'playlist', path: paths.playlists, text: 'Suas listas' },
    { label: 'Buscar no YouTube', icon: 'search', path: paths.youtube, text: 'Achar e baixar' },
    { label: 'Retrospectiva', icon: 'sparkle', path: paths.wrapped, text: 'Seu ano em música' },
  ] as const;
</script>

<header class="mb-5">
  <p class="eyebrow">{greeting}{profile.data ? `, ${profile.data.displayName.split(' ')[0]}` : ''}</p>
  <h1 class="text-3xl font-bold tracking-tight md:text-4xl">Sua discoteca</h1>
</header>

<div class="grid gap-3 lg:grid-cols-5">
  <DashCard path={contextPath} label={player.track ? 'Abrir tocando agora' : 'Abrir discoteca'} colors={palette} class="enter lg:col-span-3">
    {#snippet action(highlight)}
      <div class="flex flex-col gap-4 p-5 sm:flex-row sm:p-6">
        {#if currentPlaylist}
          <PlaylistCover playlist={currentPlaylist} size={144} />
        {:else if player.track}
          <Cover path={player.track.coverPath} thumb={player.track.thumbnailUrl} seed={player.track.id} size={144} />
        {:else if recent.length}
          <Cover path={recent[0].coverPath} thumb={recent[0].thumbnailUrl} seed={recent[0].id} size={144} />
        {:else if loading}
          <Skeleton w="144px" h="144px" />
        {:else}
          <span class="grid h-36 w-36 shrink-0 place-items-center rounded-lg bg-s3 text-lava-text"><Icon name="music" size={48} /></span>
        {/if}
        <div class="min-w-0 flex-1">
          <p class="eyebrow !text-fg/70">{player.track ? 'Tocando agora' : 'Para começar'}</p>
          <h2 class="mt-1 truncate text-2xl font-semibold">{contextName}</h2>
          {#if player.track}
            <p class="mt-2 truncate text-sm text-fg/80">{player.track.title} · {player.track.artistName}</p>
          {:else if recent.length}
            <p class="mt-2 text-sm text-fg/80">Retome a discoteca pelas faixas adicionadas recentemente.</p>
          {:else if loading}
            <Skeleton w="min(18rem, 80%)" h="1.5rem" class="mt-2" />
          {:else}
            <p class="mt-2 text-sm text-fg/80">Adicione músicas para começar a ouvir.</p>
          {/if}
          {#if player.track}
            <button class="btn btn-primary mt-4" onclick={() => { highlight(); void player.toggle(); }}>
              <Icon name={player.isPlaying || player.isLoading ? 'pause' : 'play'} size={16} fill />{player.isPlaying || player.isLoading ? 'Pausar' : 'Continuar'}
            </button>
          {:else if recent.length}
            <button class="btn btn-primary mt-4" onclick={() => { highlight(); void player.play(recent[0].id, recent.map((t) => t.id)); }}>
              <Icon name="play" size={16} fill />Tocar as recentes
            </button>
          {/if}
        </div>
      </div>
    {/snippet}
  </DashCard>

  <DashCard path={paths.stats} label="Abrir estatísticas" class="enter lg:col-span-2">
    <div class="p-5 sm:p-6">
      <p class="eyebrow !text-fg/70">Em números</p>
      <h2 class="mb-4 text-2xl font-semibold">Sua discoteca</h2>
      {#if stats}
        <dl class="grid grid-cols-2 gap-x-4 gap-y-3">
          {#each [['Faixas', fmtNumber(stats.trackCount)], ['Baixadas', fmtNumber(stats.downloadedCount)], ['Playlists', fmtNumber(stats.playlistCount)], ['Artistas', fmtNumber(stats.artistCount)]] as [label, value]}
            <div><dt class="text-xs text-fg/70">{label}</dt><dd class="tnum font-display text-2xl font-bold">{value}</dd></div>
          {/each}
        </dl>
        <p class="mt-4 text-sm text-fg/80">{fmtDuration(stats.totalListenedMs)} ouvidos</p>
      {:else if loading}
        <div class="grid grid-cols-2 gap-3" aria-label="Carregando estatísticas">
          {#each [0, 1, 2, 3] as i (i)}<div><Skeleton w="3.5rem" h="0.65rem" class="mb-2" /><Skeleton w="4.5rem" h="1.5rem" /></div>{/each}
        </div>
      {/if}
    </div>
  </DashCard>
</div>

{#if error}
  <div class="mt-5"><State kind="error" text={error} onretry={load} /></div>
{:else if stats?.trackCount === 0}
  <div class="mt-5"><State kind="empty" title="Sua discoteca está vazia" text="Busque músicas no YouTube e baixe para ouvir offline." /></div>
{/if}

<div class="mt-3 grid gap-3 lg:grid-cols-2">
  <DashCard path={paths.playlists} label="Abrir playlists" class="enter">
    <div class="min-h-56 p-5 sm:p-6">
      <p class="eyebrow !text-fg/70">Sua coleção</p>
      <h2 class="mb-4 text-2xl font-semibold">Playlists</h2>
      {#if !library.playlistsReady}
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4" aria-label="Carregando playlists">{#each [0, 1, 2, 3] as i (i)}<Skeleton w="72px" h="72px" />{/each}</div>
      {:else if library.playlistsError}
        <p class="text-sm text-muted">Não foi possível carregar as playlists. Abra Playlists para tentar de novo.</p>
      {:else if library.playlists.length}
        <ul class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {#each library.playlists.slice(0, 4) as playlist, i (playlist.id)}
            <li class="min-w-0 enter" style={enterStyle(i)}>
              <PlaylistCover {playlist} size={72} />
              <span class="mt-2 block truncate text-sm font-medium">{playlist.name}</span>
              <span class="text-xs text-muted">{playlist.trackCount} faixas</span>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="text-sm text-muted">Crie uma playlist para reunir suas músicas.</p>
      {/if}
    </div>
  </DashCard>

  <DashCard path={paths.stats} label="Abrir mais tocadas" class="enter">
    <div class={topTracks.length ? 'p-5 pb-2 sm:p-6 sm:pb-2' : 'min-h-56 p-5 sm:p-6'}>
      <p class="eyebrow !text-fg/70">No seu ritmo</p>
      <h2 class="mb-4 text-2xl font-semibold">Mais tocadas</h2>
      {#if !stats && loading}
        <div aria-label="Carregando mais tocadas" class="space-y-3">{#each [0, 1, 2] as i (i)}<Skeleton w="100%" h="2.5rem" />{/each}</div>
      {:else if !topTracks.length}
        <p class="text-sm text-muted">Ouça suas faixas para descobrir quais tocam mais.</p>
      {/if}
    </div>
    {#snippet action(highlight)}
      {#if topTracks.length}
        <!-- @req RF-14 RF-41 -->
        <ol class="px-3 pb-3 sm:px-4 sm:pb-4">
          {#each topTracks as ranked, i (ranked.track.id)}
            <li class="enter" style={enterStyle(i)}>
              <button class="flex min-h-12 w-full items-center gap-3 rounded-lg p-2 text-left hover:bg-s2" aria-label={`Tocar ${ranked.track.title}, de ${ranked.track.artistName}`} onclick={() => { highlight(); void player.play(ranked.track.id, topTracks.map((r) => r.track.id)); }}>
                <span class="tnum w-4 shrink-0 text-sm text-faint">{i + 1}</span>
                <Cover path={ranked.track.coverPath} thumb={ranked.track.thumbnailUrl} seed={ranked.track.id} size={40} />
                <span class="min-w-0 flex-1"><span class="block truncate text-sm font-medium">{ranked.track.title}</span><span class="block truncate text-xs text-muted">{ranked.track.artistName}</span></span>
                <span class="tnum shrink-0 text-xs text-faint">{ranked.plays}×</span>
              </button>
            </li>
          {/each}
        </ol>
      {/if}
    {/snippet}
  </DashCard>
</div>

<!-- @req RF-41 -->
<div class="mt-3">
  <DashCard path={paths.library} label="Abrir discoteca" class="enter">
    <div class="p-5 pb-2 sm:p-6 sm:pb-2">
      <p class="eyebrow !text-fg/70">Na sua discoteca</p>
      <h2 class="text-2xl font-semibold">Adicionadas recentemente</h2>
      {#if loading && !recent.length}
        <div aria-label="Carregando faixas recentes" class="mt-4 grid gap-3 xl:grid-cols-2">{#each [0, 1, 2, 3, 4, 5] as i (i)}<Skeleton w="100%" h="3rem" />{/each}</div>
      {:else if !recent.length}
        <p class="mt-4 text-sm text-muted">Adicione músicas para ouvir suas faixas recentes.</p>
      {/if}
    </div>
    {#snippet action(highlight)}
      {#if recent.length}
        <!-- @req RF-41 -->
        <ol class="grid gap-x-3 px-3 pb-3 sm:px-4 sm:pb-4 xl:grid-cols-2">
          {#each recent as track, i (track.id)}
            <li class="enter" style={enterStyle(i)}>
              <button class="flex min-h-12 w-full items-center gap-3 rounded-lg p-2 text-left hover:bg-s2" aria-label={`Tocar ${track.title}, de ${track.artistName}`} onclick={() => { highlight(); void player.play(track.id, recent.map((t) => t.id)); }}>
                <Cover path={track.coverPath} thumb={track.thumbnailUrl} seed={track.id} size={40} />
                <span class="min-w-0 flex-1"><span class="block truncate text-sm font-medium">{track.title}</span><span class="block truncate text-xs text-muted">{track.artistName}</span></span>
              </button>
            </li>
          {/each}
        </ol>
      {/if}
    {/snippet}
  </DashCard>
</div>

<section class="mt-7" aria-label="Atalhos">
  <h2 class="mb-3 text-xl font-semibold">Atalhos</h2>
  <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
    {#each shortcuts as shortcut, i}
      <DashCard path={shortcut.path} label={`Abrir ${shortcut.label}`} class="enter">
        <div class="flex min-h-28 items-center gap-3 p-4" style={enterStyle(i)}>
          <span class="grid h-10 w-10 shrink-0 place-items-center rounded-lg bg-s3 text-lava-text"><Icon name={shortcut.icon} size={20} /></span>
          <span><span class="block font-semibold">{shortcut.label}</span><span class="text-xs text-muted">{shortcut.text}</span></span>
        </div>
      </DashCard>
    {/each}
  </div>
</section>

<Recommendations />
