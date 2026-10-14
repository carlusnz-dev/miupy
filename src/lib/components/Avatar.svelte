<script lang="ts">
  // @req RF-21
  import { api } from '../ipc';
  import { assetCrossOrigin } from '../util/asset';
  let { path, initials, size = 40 }: { path: string | null; initials: string; size?: number } = $props();
  let failed = $state(false);
  $effect(() => { path; failed = false; });
</script>
<div class="grid shrink-0 place-items-center overflow-hidden rounded-full font-bold" style="width:{size}px;height:{size}px;background:var(--grad-lava);color:var(--lava-ink)" role="img" aria-label="Foto do perfil">
  {#if path && !failed}<img crossorigin={assetCrossOrigin(api.resolveAsset(path))} src={api.resolveAsset(path) ?? ''} alt="" class="h-full w-full object-cover" onerror={() => (failed = true)} />{:else}{initials}{/if}
</div>
