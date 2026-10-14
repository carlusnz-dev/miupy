<script lang="ts">
  // @req RNF-11
  // Esqueleto no formato do conteúdo que vai chegar, com as mesmas medidas da tela real para não
  // haver salto de layout quando os dados aparecem.
  import Skeleton from './Skeleton.svelte';

  type Props = {
    /** tracks: linhas de faixa · cards: cartões com capa · stats: números · jobs: downloads
     *  recs: sugestões · lines: texto (letra) · hero: cabeçalho com capa · form: avatar e campos
     *  card: um cartão grande (retrospectiva) */
    variant: 'tracks' | 'cards' | 'stats' | 'jobs' | 'recs' | 'lines' | 'hero' | 'form' | 'card';
    count?: number;
    /** Classe da grade, quando a tela usa colunas próprias. */
    class?: string;
    label?: string;
  };
  let { variant, count, class: cls = '', label = 'Carregando' }: Props = $props();

  const n = $derived(count ?? { tracks: 8, cards: 4, stats: 6, jobs: 3, recs: 4, lines: 7, hero: 1, form: 1, card: 1 }[variant]);
  const items = $derived(Array.from({ length: n }, (_, i) => i));
  const titleW = (i: number) => `${[46, 34, 56, 40, 50, 30, 44][i % 7]}%`;
  const subW = (i: number) => `${[24, 30, 20, 28, 22][i % 5]}%`;
</script>

<div role="status" aria-busy="true" aria-label={label} class={cls}>
  <span class="sr-only">{label}…</span>
  {#if variant === 'tracks'}
    <div>
      {#each items as i}
        <div class="flex items-center gap-2 rounded-lg px-2 py-1.5">
          <Skeleton w="44px" h="44px" rounded="0.375rem" />
          <div class="flex min-w-0 flex-1 flex-col gap-2">
            <Skeleton w={titleW(i)} h="0.8rem" />
            <Skeleton w={subW(i)} h="0.65rem" />
          </div>
          <Skeleton w="2.75rem" h="0.65rem" />
        </div>
      {/each}
    </div>
  {:else if variant === 'cards'}
    <ul class="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-4">
      {#each items as i}
        <li class="card p-3">
          <Skeleton w="100%" h="auto" rounded="0.5rem" class="mb-3 aspect-square" />
          <Skeleton w={titleW(i)} h="0.85rem" class="mb-2" />
          <Skeleton w={subW(i)} h="0.65rem" />
        </li>
      {/each}
    </ul>
  {:else if variant === 'stats'}
    <div class="grid grid-cols-2 gap-3 md:grid-cols-3">
      {#each items as i}
        <div class="card p-4">
          <Skeleton w="40%" h="0.7rem" class="mb-3" />
          <Skeleton w={`${[55, 45, 60][i % 3]}%`} h="1.9rem" />
        </div>
      {/each}
    </div>
  {:else if variant === 'jobs'}
    <ul class="flex flex-col gap-2">
      {#each items as i}
        <li class="card flex items-center gap-3 p-3">
          <Skeleton w="44px" h="44px" rounded="0.375rem" />
          <div class="flex min-w-0 flex-1 flex-col gap-2">
            <Skeleton w={titleW(i)} h="0.8rem" />
            <Skeleton w={subW(i)} h="0.65rem" />
          </div>
          <Skeleton w="2.5rem" h="0.65rem" />
        </li>
      {/each}
    </ul>
  {:else if variant === 'recs'}
    <ul class="grid gap-2 sm:grid-cols-2">
      {#each items as i}
        <li class="card flex items-center gap-3 p-3">
          <Skeleton w="56px" h="56px" rounded="0.5rem" />
          <div class="flex min-w-0 flex-1 flex-col gap-2">
            <Skeleton w={titleW(i)} h="0.85rem" />
            <Skeleton w={subW(i)} h="0.7rem" />
            <Skeleton w="35%" h="0.6rem" />
          </div>
        </li>
      {/each}
    </ul>
  {:else if variant === 'lines'}
    <div class="flex flex-col items-center gap-4 py-8">
      {#each items as i}<Skeleton w={`${[70, 55, 80, 62, 74, 48, 66][i % 7]}%`} h="1.1rem" />{/each}
    </div>
  {:else if variant === 'card'}
    <Skeleton w="100%" h="28rem" rounded="1rem" class="mx-auto max-w-xl" />
  {:else if variant === 'hero'}
    <div class="mb-6 flex flex-col items-start gap-4 rounded-2xl border border-line bg-s1 p-5 sm:flex-row sm:items-end md:p-8">
      <Skeleton w="128px" h="128px" rounded="999px" />
      <div class="flex w-full min-w-0 flex-1 flex-col gap-3">
        <Skeleton w="4rem" h="0.65rem" />
        <Skeleton w="min(22rem, 70%)" h="2.25rem" />
        <Skeleton w="min(14rem, 50%)" h="0.8rem" />
      </div>
    </div>
  {:else}
    <div class="grid max-w-3xl gap-6 md:grid-cols-[auto_1fr]">
      <div class="flex flex-col items-center gap-2"><Skeleton w="112px" h="112px" rounded="999px" /><Skeleton w="6rem" h="0.8rem" /></div>
      <div class="card flex flex-col gap-4 p-5">
        {#each [0, 1, 2, 3] as i}
          <div><Skeleton w="5rem" h="0.7rem" class="mb-2" /><Skeleton w={`${[100, 100, 100, 100][i]}%`} h="2.5rem" rounded="0.5rem" /></div>
        {/each}
      </div>
    </div>
  {/if}
</div>
