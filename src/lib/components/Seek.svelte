<script lang="ts">
  // Barra de progresso com seek. @req RF-03
  import { player } from '../stores/player.svelte';
  import { fmtTime } from '../util/format';
  let { compact = false }: { compact?: boolean } = $props();
  let dragging = $state(false);
  let dragMs = $state(0);
  const pos = $derived(dragging ? dragMs : player.state.positionMs);
  const dur = $derived(player.state.durationMs);
  const pct = $derived(dur > 0 ? (pos / dur) * 100 : 0);
</script>

<div class="flex items-center gap-2">
  {#if !compact}<span class="tnum w-10 text-right text-xs text-muted">{fmtTime(pos)}</span>{/if}
  <input
    type="range" class="slider" min="0" max={Math.max(1, dur)} step="500" value={pos}
    style="--pct:{pct}%" aria-label="Posição na faixa" aria-valuetext="{fmtTime(pos)} de {fmtTime(dur)}"
    disabled={!player.active}
    oninput={(e) => { dragging = true; dragMs = Number(e.currentTarget.value); }}
    onchange={(e) => { const v = Number(e.currentTarget.value); dragging = false; void player.seek(v); }}
  />
  {#if !compact}<span class="tnum w-10 text-xs text-muted">{fmtTime(dur)}</span>{/if}
</div>
