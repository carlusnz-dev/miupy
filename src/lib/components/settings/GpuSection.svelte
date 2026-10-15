<script lang="ts">
  // Aceleração de hardware. @req RNF-09
  import { api } from '../../ipc';
  import Dropdown from '../Dropdown.svelte';
  import { notifyError } from '../../stores/toast.svelte';

  let gpuStatus = $state<{hardware:boolean;reason:string;restartRequired:boolean;envOverride:string|null}|null>(null);
  let gpuMode = $state<'auto'|'on'|'off'>('auto');
  const gpuModes: { value: 'auto' | 'on' | 'off'; label: string }[] = [{ value: 'auto', label: 'Automático' }, { value: 'on', label: 'Ativada' }, { value: 'off', label: 'Desativada' }];
  $effect(() => {
    let alive = true;
    void api.settingsGet().then((s) => { if (alive) gpuMode = s.gpuMode; }).catch(notifyError);
    void api.systemGpuStatus().then((s) => { if (alive) gpuStatus = s; }).catch(notifyError);
    return () => { alive = false; };
  });
  async function setGpu(mode: 'auto'|'on'|'off') { try { const s=await api.settingsUpdate({settings:{gpuMode:mode}}); gpuMode=s.gpuMode; gpuStatus=await api.systemGpuStatus(); } catch(e) { notifyError(e); } }
</script>

<section class="mb-8 card p-5" aria-labelledby="h-gpu">
  <h2 id="h-gpu" class="text-lg font-semibold">Aceleração de hardware</h2>
  <label class="mt-3 block text-sm" for="gpu-mode">Modo</label>
  <Dropdown id="gpu-mode" class="mt-1" value={gpuMode} options={gpuModes} onchange={setGpu} />
  {#if gpuStatus}<p class="mt-3 text-sm text-muted">{gpuStatus.hardware ? 'hardware' : 'software'} · {gpuStatus.reason}</p>{/if}
  {#if gpuMode === 'on'}<p class="mt-2 text-sm text-muted">Em Wayland com NVIDIA, esta opção pode impedir o app de abrir.</p>{/if}
  {#if gpuStatus?.restartRequired}<p class="mt-1 text-sm">Reinicie o app para aplicar.</p>{/if}
</section>
