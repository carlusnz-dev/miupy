<script lang="ts">
  // Configurações: seletor da cor da lava com pré-visualização. @req RF-17 RNF-05
  import type { LavaTheme } from '../lib/ipc';
  import { api, type YtDlpStatus } from '../lib/ipc';
  import { isMock } from '../lib/ipc';
  import { settings } from '../lib/stores/settings.svelte';
  import { notifyError } from '../lib/stores/toast.svelte';
  import PageHeader from '../lib/components/PageHeader.svelte';
  import MeshGradient from '../lib/components/MeshGradient.svelte';
  import Icon from '../lib/components/Icon.svelte';

  let ytdlp = $state<YtDlpStatus | null>(null);
  let metadata = $state<{ done: number; total: number } | null>(null);
  let onlyMissing = $state(true);
  let updating = $state(false);
  let refreshing = $state(false);
  let gpuStatus = $state<{hardware:boolean;reason:string;restartRequired:boolean;envOverride:string|null}|null>(null);
  let gpuMode = $state<'auto'|'on'|'off'>('auto');
  $effect(() => {
    let alive = true; let unY: (() => void) | undefined; let unM: (() => void) | undefined;
    void api.ytdlpStatus().then((s) => { if (alive) ytdlp = s; }).catch(notifyError);
    void api.settingsGet().then((s) => { if (alive) gpuMode = s.gpuMode; }).catch(notifyError);
    void api.systemGpuStatus().then((s) => { if (alive) gpuStatus = s; }).catch(notifyError);
    void api.on('ytdlp://status', (s) => { if (alive) ytdlp = s; }).then((un) => { if (alive) unY = un; else un(); });
    void api.on('metadata://progress', (p) => { if (alive) metadata = p; }).then((un) => { if (alive) unM = un; else un(); });
    return () => { alive = false; unY?.(); unM?.(); };
  }); // @req RF-26 RF-12
  async function updateYtdlp() { updating = true; try { ytdlp = await api.ytdlpUpdate(); } catch (e) { notifyError(e); } finally { updating = false; } }
  async function refreshMetadata() { refreshing = true; metadata = null; try { const total = await api.metadataRefreshLibrary({ onlyMissing }); if (total === 0) metadata = { done: 0, total: 0 }; } catch (e) { notifyError(e); } finally { refreshing = false; } }
  const options: { id: LavaTheme; name: string; text: string }[] = [
    { id: 'orange', name: 'Laranja e vermelho', text: 'Lava clássica (padrão)' },
    { id: 'purple', name: 'Roxo', text: 'Magma violeta' },
    { id: 'blue', name: 'Azul', text: 'Lava azul' },
  ];
  const ramp = 'linear-gradient(135deg, var(--lava-core), var(--lava) 45%, var(--lava-deep))';

  async function pick(id: LavaTheme) {
    try { await settings.setLava(id); } catch (e) { notifyError(e); }
  }
  async function setGpu(mode: 'auto'|'on'|'off') { try { const s=await api.settingsUpdate({settings:{gpuMode:mode}}); gpuMode=s.gpuMode; gpuStatus=await api.systemGpuStatus(); } catch(e) { notifyError(e); } }
  const keys = [['Espaço', 'tocar/pausar'], ['← →', 'voltar/avançar 5 s'], ['↑ ↓', 'volume'], ['M', 'silenciar']];
</script>

<PageHeader title="Configurações" />

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

<section class="mb-8 card p-5" aria-labelledby="h-ytdlp">
  <h2 id="h-ytdlp" class="text-lg font-semibold">yt-dlp</h2>
  <p class="mb-2 text-sm text-muted">{ytdlp?.state === 'ready' ? `Pronto · versão ${ytdlp.version ?? 'desconhecida'}` : ytdlp?.state === 'downloading' ? 'Baixando atualização…' : ytdlp?.state === 'error' ? 'Erro' : 'Não instalado'}</p>
  {#if ytdlp?.state === 'downloading'}<progress max="1" value={ytdlp.progress} class="w-full" aria-label="Progresso do yt-dlp"></progress>{/if}
  {#if ytdlp?.error}<p class="text-sm text-danger">{ytdlp.error}</p>{/if}
  <button class="btn mt-3" disabled={updating || ytdlp?.state === 'downloading'} onclick={updateYtdlp}>Procurar atualização</button>
</section>
<section class="mb-8 card p-5" aria-labelledby="h-gpu">
  <h2 id="h-gpu" class="text-lg font-semibold">Aceleração de hardware</h2>
  <label class="mt-3 block text-sm">Modo
    <select class="input mt-1 block" value={gpuMode} onchange={(e) => setGpu(e.currentTarget.value as 'auto'|'on'|'off')}>
      <option value="auto">Automático</option><option value="on">Ativada</option><option value="off">Desativada</option>
    </select>
  </label>
  {#if gpuStatus}<p class="mt-3 text-sm text-muted">{gpuStatus.hardware ? 'hardware' : 'software'} · {gpuStatus.reason}</p>{/if}
  <!-- @req RNF-09 -->
  {#if gpuMode === 'on'}<p class="mt-2 text-sm text-muted">Em Wayland com NVIDIA, esta opção pode impedir o app de abrir.</p>{/if}
  {#if gpuStatus?.restartRequired}<p class="mt-1 text-sm">Reinicie o app para aplicar.</p>{/if}
</section>
<section class="mb-8 card p-5" aria-labelledby="h-metadata">
  <h2 id="h-metadata" class="text-lg font-semibold">Metadados</h2>
  <label class="my-3 flex items-center gap-2 text-sm"><input type="checkbox" bind:checked={onlyMissing} />Reprocessar apenas dados ausentes</label>
  <button class="btn" disabled={refreshing} onclick={refreshMetadata}>Reprocessar metadados</button>
  {#if metadata}<p class="mt-2 text-sm text-muted" role="status">{metadata.total ? `${metadata.done} de ${metadata.total} faixas` : 'Nenhuma faixa para reprocessar.'}</p>{/if}
</section>
<section aria-labelledby="h-keys" class="hidden md:block">
  <h2 id="h-keys" class="mb-2 text-lg font-semibold">Atalhos de teclado</h2>
  <dl class="card grid max-w-md grid-cols-[auto_1fr] gap-x-6 gap-y-2 p-4 text-sm">
    {#each keys as [k, d]}<dt><kbd class="chip">{k}</kbd></dt><dd class="text-muted">{d}</dd>{/each}
  </dl>
</section>

{#if isMock}
  <p class="mt-8 text-xs text-faint">Rodando com dados de demonstração (mock em memória). Dentro do app Tauri, os dados vêm do backend.</p>
{/if}
