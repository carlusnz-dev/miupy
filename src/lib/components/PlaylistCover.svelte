<script lang="ts">
  // @req RF-22
  import type { Playlist } from '../ipc';
  import { api } from '../ipc';
  import { assetCrossOrigin } from '../util/asset';
  import { mosaicCells } from '../util/mosaic';
  import Cover from './Cover.svelte';
  let { playlist, size, class: cls = '' }: { playlist: Playlist; size?: number; class?: string } = $props();
  const cells = $derived(mosaicCells(playlist.mosaicPaths));
</script>
{#if playlist.coverPath || cells.length === 0}
  <Cover path={playlist.coverPath} seed={playlist.id} {size} class={cls} label="Capa de {playlist.name}" />
{:else if cells.length === 1}
  <Cover path={cells[0]} seed={playlist.id} {size} class={cls} label="Capa de {playlist.name}" />
{:else}
  <div class="grid shrink-0 grid-cols-2 overflow-hidden rounded-lg bg-s3 {cls}" style={size ? `width:${size}px;height:${size}px` : 'aspect-ratio:1/1'} role="img" aria-label="Mosaico da playlist {playlist.name}">
    {#each cells as path, i}
      <img crossorigin={assetCrossOrigin(api.resolveAsset(path))} src={api.resolveAsset(path) ?? ''} alt="" class="h-full w-full object-cover" />
    {/each}
  </div>
{/if}
