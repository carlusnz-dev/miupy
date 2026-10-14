<script lang="ts">
  // @req RF-02
  import { player } from '../stores/player.svelte';
  import Icon from './Icon.svelte';
  const v = $derived(player.state.muted ? 0 : player.state.volume);
</script>

<div class="flex items-center gap-1">
  <button class="icon-btn" aria-label={player.state.muted ? 'Ativar o som' : 'Silenciar'} aria-pressed={player.state.muted} onclick={() => player.toggleMuted()}>
    <Icon name={v === 0 ? 'mute' : 'volume'} size={20} />
  </button>
  <input
    type="range" class="slider !w-24" min="0" max="1" step="0.01" value={v} style="--pct:{v * 100}%"
    aria-label="Volume" aria-valuetext="{Math.round(v * 100)}%"
    oninput={(e) => { const x = Number(e.currentTarget.value); if (player.state.muted && x > 0) void player.setMuted(false); void player.setVolume(x); }}
  />
</div>
