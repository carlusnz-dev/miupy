<script lang="ts">
  // @req RF-24
  import { player } from '../stores/player.svelte';
  import { shuffleLabel } from '../util/shuffle';
  import Icon from './Icon.svelte';
  let { showText = false }: { showText?: boolean } = $props();
</script>
<button class="icon-btn {player.state.shuffle !== 'off' ? 'on' : ''}" aria-label={shuffleLabel(player.state.shuffle)} title={shuffleLabel(player.state.shuffle)} aria-pressed={player.state.shuffle !== 'off'} onclick={() => player.cycleShuffle()}>
  <Icon name={player.state.shuffle === 'smart' ? 'sparkle' : 'shuffle'} size={20} />
  {#if showText}<span class="text-xs">{shuffleLabel(player.state.shuffle)}</span>{/if}
</button>
