<script lang="ts">
  import AnimatedText from './AnimatedText.svelte';
  type Props = { title: string; subtitle?: string; eyebrow?: string; actions?: import('svelte').Snippet };
  let { title, subtitle, eyebrow, actions }: Props = $props();
</script>

<header class="mb-5 flex flex-wrap items-end justify-between gap-3">
  <div class="min-w-0">
    {#if eyebrow}<p class="eyebrow mb-1">{eyebrow}</p>{/if}
    <h1 class="truncate text-2xl font-bold tracking-tight md:text-3xl"><AnimatedText text={title} /></h1>
    {#if subtitle}<p class="mt-1 text-sm text-muted">{subtitle}</p>{/if}
  </div>
  {#if actions}<div class="flex flex-wrap items-center gap-2">{@render actions()}</div>{/if}
</header>
