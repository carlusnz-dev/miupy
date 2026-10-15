<script lang="ts">
  // Artista: dados de metadata_get_artist + faixas dele na discoteca. @req RF-12 RF-05 RF-42 RNF-11 RNF-13
  import { api, type ArtistInfo, type Track } from '../lib/ipc';
  import { mergeDeep, reconcileList } from '../lib/util/diff';
  import { Chunked } from '../lib/util/chunked.svelte';
  import { Resource } from '../lib/stores/resource.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { href, paths } from '../lib/stores/router.svelte';
  import TrackRow from '../lib/components/TrackRow.svelte';
  import Cover from '../lib/components/Cover.svelte';
  import MeshGradient from '../lib/components/MeshGradient.svelte';
  import State from '../lib/components/State.svelte';
  import SkeletonList from '../lib/components/SkeletonList.svelte';
  import Icon from '../lib/components/Icon.svelte';

  let { id }: { id: string } = $props();
  type ArtistData = { info: ArtistInfo; tracks: Track[] };
  const res = new Resource<ArtistData>(['artist', 'tracks'], (prev, next) => {
    const info = mergeDeep(prev?.info, next.info);
    const tracks = reconcileList(prev?.tracks, next.tracks, (t) => t.id);
    return prev && prev.info === info && prev.tracks === tracks ? prev : { info, tracks };
  });
  // svelte-ignore state_referenced_locally
  const key = `artist:${id}`;
  res.prime(key);
  const info = $derived(res.data?.info ?? null);
  const tracks = $derived(res.data?.tracks ?? []);
  const loading = $derived(res.loading);
  const error = $derived(res.data === undefined ? res.error : null);
  $effect(() => res.attach());
  async function load() {
    await res.load(key, async () => {
      const [info, tracks] = await Promise.all([
        api.metadataGetArtist({ artistId: id }),
        api.libraryListTracksByArtist({ artistId: id, query: { sortBy: 'album', sortDir: 'asc' } }),
      ]);
      return { info, tracks };
    });
  }
  $effect(() => { void load(); });
  const queue = $derived(tracks.map((t) => t.id));
  const context = $derived({ kind: 'artist' as const, id, label: info?.name ?? '' });
  const chunk = new Chunked();
  $effect(() => chunk.advance(tracks.length));
  const rows = $derived(tracks.slice(0, chunk.shown));
</script>

<a class="mb-3 inline-flex items-center gap-1 text-sm text-muted hover:text-fg" href={href(paths.library)}><Icon name="left" size={16} />Discoteca</a>

{#if loading}
  <SkeletonList variant="hero" label="Carregando o artista" />
  <SkeletonList variant="tracks" count={6} />
{:else if error || !info}
  <State kind="error" title="Artista não encontrado" text={error ?? undefined} onretry={load} />
{:else}
  <MeshGradient class="mb-6 rounded-2xl border border-line" intensity={0.85}>
    <div class="flex flex-col items-start gap-4 p-5 sm:flex-row sm:items-end md:p-8">
      <div><Cover path={info.imagePath} seed={info.id} size={128} rounded="999px" />{#if info.imagePath && info.imageCredit}<p class="mt-1 max-w-32 text-center text-xs text-faint">{info.imageCredit}</p>{/if}</div>
      <div class="min-w-0">
        <p class="eyebrow !text-fg/70">Artista</p>
        <h1 class="text-3xl font-bold tracking-tight md:text-5xl">{info.name}</h1>
        <p class="mt-2 flex flex-wrap items-center gap-1.5 text-sm text-fg/80">
          <span>{info.trackCount} {info.trackCount === 1 ? 'faixa' : 'faixas'}</span>
          {#if info.country}<span class="chip">{info.country}</span>{/if}
          {#each info.genres as g}<span class="chip">{g}</span>{/each}
        </p>
      </div>
    </div>
  </MeshGradient>

  {#if info.bio}
    <p class="mb-6 max-w-2xl text-sm leading-relaxed text-muted">{info.bio}</p>
  {:else}
    <p class="mb-6 text-sm text-faint">Sem biografia. Os dados do artista vêm do MusicBrainz quando há rede.</p>
  {/if}

  <div class="mb-3 flex items-center gap-2">
    <h2 class="text-lg font-semibold">Faixas</h2>
    <button class="btn btn-primary btn-sm ml-auto" disabled={!tracks.length} onclick={() => player.play(tracks[0].id, queue, context)}><Icon name="play" size={14} fill />Tocar</button>
  </div>
  {#if tracks.length === 0}
    <State kind="empty" title="Nenhuma faixa deste artista na discoteca" />
  {:else}
    <div role="list">
      {#each rows as t, i (t.id)}
        <TrackRow track={t} index={i} current={player.track?.id === t.id} playing={player.isPlaying} onplay={() => player.play(t.id, queue, context)} />
      {/each}
    </div>
    {#if chunk.shown < tracks.length}<div aria-hidden="true" style="height:{chunk.rest(tracks.length)}px"></div>{/if}
  {/if}
{/if}
