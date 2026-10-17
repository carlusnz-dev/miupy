<script lang="ts">
  // Capa da faixa atual desfocada, para ficar por baixo de um vidro ou como brilho ambiente. O pai
  // posiciona e recorta (relative + overflow-hidden); a força vem de --cover-blur e --cover-glow-a.
  // @req RF-40 RNF-04
  import { cover } from '../stores/cover.svelte';
  import { assetCrossOrigin } from '../util/asset';
</script>

{#key cover.src}
  {#if cover.src}<img class="cover-glow" src={cover.src} crossorigin={assetCrossOrigin(cover.src)} alt="" aria-hidden="true" decoding="async" />{/if}
{/key}
