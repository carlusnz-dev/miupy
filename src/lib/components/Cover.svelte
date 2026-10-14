<script lang="ts">
  import { api } from '../ipc';
  import { hash } from '../util/format';
  import Icon from './Icon.svelte';

  type Props = { path?: string | null; url?: string | null; seed: string; size?: number | string; rounded?: string; class?: string; label?: string };
  let { path = null, url = null, seed, size, rounded = '0.5rem', class: cls = '', label = '' }: Props = $props();

  const src = $derived(url ?? api.resolveAsset(path));
  let failed = $state(false);
  $effect(() => { src; failed = false; });
  const h = $derived(hash(seed));
  const bg = $derived(
    `radial-gradient(90% 90% at ${h % 90}% ${(h >> 8) % 90}%, var(--lava-core) 0%, transparent 62%), linear-gradient(${(h >> 4) % 360}deg, var(--lava-deep), var(--lava) 70%, var(--lava-ember))`,
  );
  const dim = $derived(typeof size === 'number' ? `${size}px` : size);
</script>

<div
  class="relative grid shrink-0 place-items-center overflow-hidden bg-s3 {cls}"
  style="border-radius:{rounded};{dim ? `width:${dim};height:${dim};` : ''}aspect-ratio:1/1"
  role={label ? 'img' : undefined}
  aria-label={label || undefined}
  aria-hidden={label ? undefined : true}
>
  {#if src && !failed}
    <img {src} alt="" class="h-full w-full object-cover" loading="lazy" onerror={() => (failed = true)} />
  {:else}
    <div class="absolute inset-0" style="background:{bg}"></div>
    <Icon name="music" size={typeof size === 'number' ? Math.max(14, size * 0.4) : 28} class="relative text-lava-ink opacity-70" />
  {/if}
</div>
