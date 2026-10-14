<script lang="ts">
  // @req RNF-08
  // Botões de transporte reutilizados no player de barra e no "tocando agora". @req RF-01 RF-03
  import { player } from '../stores/player.svelte';
  import Icon from './Icon.svelte';
  let { big = false }: { big?: boolean } = $props();
</script>

<div class="flex items-center {big ? 'gap-4' : 'gap-1'}">
  <button class="icon-btn {big ? '!h-12 !w-12' : ''}" aria-label="Faixa anterior" disabled={!player.active} onclick={() => player.previous()}>
    <Icon name="prev" size={big ? 26 : 20} />
  </button>
  <button
    class="btn-primary motion-bounce grid shrink-0 place-items-center rounded-full border-0 {big ? 'h-16 w-16' : 'h-10 w-10'}"
    aria-label={player.isPlaying ? 'Pausar' : 'Tocar'} disabled={!player.active} onclick={() => player.toggle()}
  >
    <Icon name={player.isPlaying || player.isLoading ? 'pause' : 'play'} size={big ? 30 : 22} fill />
  </button>
  <button class="icon-btn {big ? '!h-12 !w-12' : ''}" aria-label="Próxima faixa" disabled={!player.hasNext} onclick={() => player.next()}>
    <Icon name="next" size={big ? 26 : 20} />
  </button>
</div>
