<script lang="ts">
  // @req RF-22
  import type { Playlist } from '../ipc';
  import { api } from '../ipc';
  import { assetCrossOrigin } from '../util/asset';
  import { mosaicCells } from '../util/mosaic';
  import Cover from './Cover.svelte';
  import { useShared as sharedAction } from '../layoutPresence';
  let { playlist, size, class: cls = '', shared }: { playlist: Playlist; size?: number; class?: string; shared?: string } = $props();
  const cells = $derived(mosaicCells(playlist.mosaicPaths));
</script>
{#if playlist.coverPath || cells.length === 0}
  <Cover path={playlist.coverPath} seed={playlist.id} {size} class={cls} label="Capa de {playlist.name}" {shared} />
{:else if cells.length === 1}
  <Cover path={cells[0]} seed={playlist.id} {size} class={cls} label="Capa de {playlist.name}" {shared} />
{:else}
  <div class="grid shrink-0 grid-cols-2 overflow-hidden rounded-lg bg-s3 {cls}" style={size ? `width:${size}px;height:${size}px` : 'aspect-ratio:1/1'} role="img" aria-label="Mosaico da playlist {playlist.name}" use:sharedAction={shared ?? ''}>
    {#each cells as path, i}
      <img crossorigin={assetCrossOrigin(api.resolveAsset(path))} src={api.resolveAsset(path) ?? ''} alt="" class="h-full w-full object-cover" />
    {/each}
  </div>
{/if}
