<script lang="ts">
  // @req RF-28
  import type { Normalization } from '../../ipc';
  import { settings } from '../../stores/settings.svelte';
  import { notifyError } from '../../stores/toast.svelte';

  const modes: { id: Normalization; name: string; description: string }[] = [
    { id: 'off', name: 'Desligado', description: 'Mantém o volume original de cada faixa.' },
    { id: 'quiet', name: 'Baixo', description: 'Alvo de −19 LUFS, bom para ouvir por mais tempo.' },
    { id: 'normal', name: 'Normal', description: 'Alvo de −14 LUFS, equilíbrio padrão.' },
    { id: 'loud', name: 'Alto', description: 'Alvo de −11 LUFS, com limitação de picos.' },
  ];

  async function choose(mode: Normalization) {
    try { await settings.setNormalization(mode); } catch (error) { notifyError(error); }
  }
</script>

<section class="mb-8" aria-labelledby="h-audio">
  <h2 id="h-audio" class="text-lg font-semibold">Áudio</h2>
  <h3 id="h-normalization" class="mt-3 mb-2 font-medium">Normalização de volume</h3>
  <p class="mb-3 text-sm text-muted">Equilibra o volume percebido entre as músicas.</p>
  <div class="grid gap-2 sm:grid-cols-2" role="radiogroup" aria-labelledby="h-normalization">
    {#each modes as mode}
      <button type="button" role="radio" aria-checked={settings.normalization === mode.id}
        onclick={() => choose(mode.id)} class="card p-3 text-left {settings.normalization === mode.id ? '!border-lava-text' : 'hover:border-line-strong'}">
        <span class="block font-medium">{mode.name}</span>
        <span class="text-sm text-muted">{mode.description}</span>
      </button>
    {/each}
  </div>
</section>
