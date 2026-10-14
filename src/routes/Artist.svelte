<script lang="ts">
  // Artista: dados de metadata_get_artist + faixas dele na biblioteca. @req RF-12 RF-05
  import { api, errorMessage, type ArtistInfo, type Track } from '../lib/ipc';
  import { library } from '../lib/stores/library.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { href, paths } from '../lib/stores/router.svelte';
  import TrackRow from '../lib/components/TrackRow.svelte';
  import Cover from '../lib/components/Cover.svelte';
  import MeshGradient from '../lib/components/MeshGradient.svelte';
  import State from '../lib/components/State.svelte';
  import Icon from '../lib/components/Icon.svelte';

  let { id }: { id: string } = $props();
  let info = $state<ArtistInfo | null>(null);
  let tracks = $state<Track[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    error = null;
    try {
      [info, tracks] = await Promise.all([
        api.metadataGetArtist({ artistId: id }),
        api.libraryListTracks({ query: { sortBy: 'album', sortDir: 'asc' } }).then((l) => l.filter((t) => t.artistId === id)),
      ]);
    } catch (e) { error = errorMessage(e); } finally { loading = false; }
  }
  $effect(() => { library.version; void load(); });
  const queue = $derived(tracks.map((t) => t.id));
</script>

<a class="mb-3 inline-flex items-center gap-1 text-sm text-muted hover:text-fg" href={href(paths.library)}><Icon name="left" size={16} />Biblioteca</a>

{#if loading}
  <State kind="loading" />
{:else if error || !info}
  <State kind="error" title="Artista não encontrado" text={error ?? undefined} onretry={load} />
{:else}
  <MeshGradient class="mb-6 rounded-2xl border border-line" intensity={0.85}>
    <div class="flex flex-col items-start gap-4 p-5 sm:flex-row sm:items-end md:p-8">
      <Cover path={info.imagePath} seed={info.id} size={128} rounded="999px" />
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
    <button class="btn btn-primary btn-sm ml-auto" disabled={!tracks.length} onclick={() => player.play(tracks[0].id, queue)}><Icon name="play" size={14} fill />Tocar</button>
  </div>
  {#if tracks.length === 0}
    <State kind="empty" title="Nenhuma faixa deste artista na biblioteca" />
  {:else}
    <div role="list">
      {#each tracks as t, i (t.id)}
        <TrackRow track={t} index={i} current={player.track?.id === t.id} playing={player.isPlaying} onplay={() => player.play(t.id, queue)} />
      {/each}
    </div>
  {/if}
{/if}
