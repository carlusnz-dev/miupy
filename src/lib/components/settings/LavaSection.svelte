<script lang="ts">
  // Cor da lava com pré-visualização. @req RF-17 RNF-05
  import type { LavaTheme } from '../../ipc';
  import { settings } from '../../stores/settings.svelte';
  import { notifyError } from '../../stores/toast.svelte';
  import MeshGradient from '../MeshGradient.svelte';
  import Icon from '../Icon.svelte';

  const options: { id: LavaTheme; name: string; text: string }[] = [
    { id: 'orange', name: 'Laranja e vermelho', text: 'Lava clássica (padrão)' },
    { id: 'purple', name: 'Roxo', text: 'Magma violeta' },
    { id: 'blue', name: 'Azul', text: 'Lava azul' },
  ];
  const ramp = 'linear-gradient(135deg, var(--lava-core), var(--lava) 45%, var(--lava-deep))';

  async function pick(id: LavaTheme) {
    try { await settings.setLava(id); } catch (e) { notifyError(e); }
  }
</script>

<section class="mb-8" aria-labelledby="h-lava">
  <h2 id="h-lava" class="text-lg font-semibold">Cor da lava</h2>
  <p class="mb-3 text-sm text-muted">Muda o destaque e os gradientes do app inteiro, na hora.</p>
  <div class="grid gap-3 md:grid-cols-3" role="radiogroup" aria-labelledby="h-lava">
    {#each options as o}
      <button role="radio" aria-checked={settings.lava === o.id} data-lava-preview={o.id} onclick={() => pick(o.id)}
        class="card overflow-hidden p-0 text-left {settings.lava === o.id ? '!border-lava-text' : 'hover:border-line-strong'}">
        <div class="h-24" style="background:{ramp}"></div>
        <div class="flex items-center gap-3 p-3">
          <span class="grid h-5 w-5 place-items-center rounded-full border {settings.lava === o.id ? 'border-transparent' : 'border-line-strong'}" style={settings.lava === o.id ? `background:${ramp};color:var(--lava-ink)` : ''}>
            {#if settings.lava === o.id}<Icon name="check" size={12} />{/if}
          </span>
          <span><span class="block font-medium">{o.name}</span><span class="text-xs text-muted">{o.text}</span></span>
        </div>
      </button>
    {/each}
  </div>
</section>

<section class="mb-8" aria-labelledby="h-prev">
  <h2 id="h-prev" class="mb-3 text-lg font-semibold">Pré-visualização</h2>
  <MeshGradient class="rounded-2xl border border-line">
    <div class="flex flex-col gap-4 p-6 md:flex-row md:items-center md:p-8">
      <div class="flex-1">
        <p class="eyebrow !text-fg/70">Tocando agora</p>
        <p class="text-2xl font-bold">Cratera de Neon</p>
        <p class="text-fg/80">Aurora Basalto</p>
        <div class="mt-4 h-1 max-w-sm rounded-full bg-black/40"><div class="h-full w-2/5 rounded-full" style="background:var(--grad-lava)"></div></div>
      </div>
      <div class="flex gap-2"><span class="btn btn-primary" aria-hidden="true">Botão primário</span><span class="btn" aria-hidden="true">Secundário</span></div>
    </div>
  </MeshGradient>
</section>
