<script lang="ts">
  // @req RF-28
  import { api, EQ_BANDS_HZ, type EqPreset, type Normalization } from '../../ipc';
  import { settings } from '../../stores/settings.svelte';
  import { notifyError } from '../../stores/toast.svelte';

  const modes: { id: Normalization; name: string; description: string }[] = [
    { id: 'off', name: 'Desligado', description: 'Mantém o volume original de cada faixa.' },
    { id: 'quiet', name: 'Baixo', description: 'Alvo de −19 LUFS, bom para ouvir por mais tempo.' },
    { id: 'normal', name: 'Normal', description: 'Alvo de −14 LUFS, equilíbrio padrão.' },
    { id: 'loud', name: 'Alto', description: 'Alvo de −11 LUFS, com limitação de picos.' },
  ];

  // @req RF-29
  let presets = $state<EqPreset[]>([]);
  let editId = $state<string | undefined>();
  let name = $state('');
  let gains = $state<number[]>(Array(8).fill(0));
  let busy = $state(false);
  $effect(() => { void api.equalizerListPresets().then(v => presets=v).catch(notifyError); });
  function edit(p?: EqPreset) { editId=p?.id; name=p?.name ?? ''; gains=[...(p?.gainsDb ?? Array(8).fill(0))]; }
  async function select(id:string) { try { await settings.setEqualizer(id); } catch(e) { notifyError(e); } }
  async function save() { busy=true; try { const saved=await api.equalizerSavePreset({preset:{id:editId,name,gainsDb:gains}}); presets=await api.equalizerListPresets(); edit(saved); } catch(e) {notifyError(e);} finally {busy=false;} }
  async function remove() { if(!editId)return; busy=true; try { await api.equalizerDeletePreset({presetId:editId}); presets=await api.equalizerListPresets(); if(settings.eqPresetId===editId) settings.eqPresetId='flat'; edit(); } catch(e) {notifyError(e);} finally {busy=false;} }

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
  <h3 class="mt-6 mb-2 font-medium">Equalizador</h3>
  <p class="mb-3 text-sm text-muted">Ajuste o som com um perfil pronto ou crie até três perfis.</p>
  <label class="block text-sm" for="eq-preset">Perfil ativo</label>
  <select id="eq-preset" class="input mt-1 w-full" value={settings.eqPresetId} onchange={(e) => select(e.currentTarget.value)}>
    {#each presets as preset}<option value={preset.id}>{preset.name}</option>{/each}
  </select>
  <div class="mt-4 flex flex-wrap gap-2">
    {#each presets.filter(p => !p.builtin) as preset}
      <button type="button" class="btn" onclick={() => edit(preset)}>{preset.name}</button>
    {/each}
    {#if presets.length < 11}<button type="button" class="btn" onclick={() => edit()}>Criar perfil</button>{/if}
  </div>
  {#if editId || presets.length < 11}
    <div class="card mt-4 space-y-3 p-4">
      <label class="block text-sm" for="eq-name">Nome do perfil</label>
      <input id="eq-name" class="input w-full" maxlength="30" bind:value={name} placeholder="Meu perfil" />
      {#each EQ_BANDS_HZ as hz, i}
        <label class="flex items-center gap-3 text-sm">
          <span class="w-16">{hz >= 1000 ? `${hz/1000} kHz` : `${hz} Hz`}</span>
          <input type="range" min="-12" max="12" step="0.5" value={gains[i]} oninput={(e) => { gains[i]=Number(e.currentTarget.value); gains=[...gains]; }} class="min-w-0 flex-1" />
          <span class="w-12 text-right">{gains[i]} dB</span>
        </label>
      {/each}
      <div class="flex gap-2">
        <button type="button" class="btn btn-primary" disabled={busy || !name.trim()} onclick={save}>Salvar perfil</button>
        {#if editId}<button type="button" class="btn" disabled={busy} onclick={remove}>Apagar</button>{/if}
      </div>
    </div>
  {/if}
</section>
