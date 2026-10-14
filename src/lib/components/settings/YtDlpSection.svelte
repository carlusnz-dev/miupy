<script lang="ts">
  // yt-dlp gerenciado pelo app. @req RF-26
  import { api, type YtDlpStatus } from '../../ipc';
  import { notifyError } from '../../stores/toast.svelte';

  let ytdlp = $state<YtDlpStatus | null>(null);
  let updating = $state(false);
  $effect(() => {
    let alive = true; let un: (() => void) | undefined;
    void api.ytdlpStatus().then((s) => { if (alive) ytdlp = s; }).catch(notifyError);
    void api.on('ytdlp://status', (s) => { if (alive) ytdlp = s; }).then((u) => { if (alive) un = u; else u(); });
    return () => { alive = false; un?.(); };
  });
  async function updateYtdlp() { updating = true; try { ytdlp = await api.ytdlpUpdate(); } catch (e) { notifyError(e); } finally { updating = false; } }
</script>

<section class="mb-8 card p-5" aria-labelledby="h-ytdlp">
  <h2 id="h-ytdlp" class="text-lg font-semibold">yt-dlp</h2>
  <p class="mb-2 text-sm text-muted">{ytdlp?.state === 'ready' ? `Pronto · versão ${ytdlp.version ?? 'desconhecida'}` : ytdlp?.state === 'downloading' ? 'Baixando atualização…' : ytdlp?.state === 'error' ? 'Erro' : 'Não instalado'}</p>
  {#if ytdlp?.state === 'downloading'}<progress max="1" value={ytdlp.progress} class="w-full" aria-label="Progresso do yt-dlp"></progress>{/if}
  {#if ytdlp?.error}<p class="text-sm text-danger">{ytdlp.error}</p>{/if}
  <button class="btn mt-3" disabled={updating || ytdlp?.state === 'downloading'} onclick={updateYtdlp}>Procurar atualização</button>
</section>
