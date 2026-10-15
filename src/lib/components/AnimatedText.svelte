<script lang="ts">
  // Texto que entra letra por letra (fade + Y, entrada padrão `.enter`) e reanima quando muda.
  // Leitores de tela leem só o texto `sr-only` (aria-label em <span> sem papel não é anunciado);
  // palavras não quebram no meio.
  // @req RNF-08
  import { letterDelays, splitWords } from '../util/animatedText';
  let { text, class: cls = '' }: { text: string; class?: string } = $props();

  const words = $derived(splitWords(text));
  const delays = $derived(letterDelays(words));
</script>

{#key text}
  <span class={cls}><span class="sr-only">{text}</span><span aria-hidden="true">{#each words as word, w}{#if w > 0}{' '}{/if}<span class="inline-block whitespace-nowrap">{#each word as letter, l}<span class="enter inline-block" style="--enter-delay:{delays[w][l]}ms">{letter}</span>{/each}</span>{/each}</span></span>
{/key}
