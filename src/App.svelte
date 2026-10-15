<script lang="ts">
  // @req RNF-08
  // Casca da aplicação: layout responsivo, inicialização das stores, atalhos de teclado.
  // @req RNF-03 RNF-04
  import { onMount } from 'svelte';
  import { springTransition, springCollapse } from './lib/motion';
  import { captureSharedLayout } from './lib/layoutPresence';
  import { api, isMock, type MetadataRefreshStatus } from './lib/ipc';
  import { notifyError, toasts } from './lib/stores/toast.svelte';
  import { router } from './lib/stores/router.svelte';
  import { player } from './lib/stores/player.svelte';
  import { library } from './lib/stores/library.svelte';
  import { downloads } from './lib/stores/downloads.svelte';
  import { settings } from './lib/stores/settings.svelte';
  import { profile } from './lib/stores/profile.svelte';
  import Nav from './lib/components/Nav.svelte';
  import PlayerBar from './lib/components/PlayerBar.svelte';
  import MiniPlayer from './lib/components/MiniPlayer.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import PinnedToast from './lib/components/PinnedToast.svelte';
  import Dialogs from './lib/components/Dialogs.svelte';
  import State from './lib/components/State.svelte';
  import Home from './routes/Home.svelte';
  import Library from './routes/Library.svelte';
  import Playlists from './routes/Playlists.svelte';
  import PlaylistDetail from './routes/PlaylistDetail.svelte';
  import YouTube from './routes/YouTube.svelte';
  import Downloads from './routes/Downloads.svelte';
  import NowPlaying from './routes/NowPlaying.svelte';
  import Artist from './routes/Artist.svelte';
  import Stats from './routes/Stats.svelte';
  import Dna from './routes/Dna.svelte';
  import Wrapped from './routes/Wrapped.svelte';
  import Profile from './routes/Profile.svelte';
  import SettingsPage from './routes/Settings.svelte';

  // @req RF-32
  let metadata = $state<MetadataRefreshStatus | null>(null);
  let hiddenBatch = $state<number | null>(null);
  onMount(() => {
    // @req RNF-09: só confirma quando a interface montou e permaneceu aberta por cinco segundos.
    const gpuTimer = window.setTimeout(() => { void api.systemGpuStartupConfirmed().catch(console.error); }, 5000);
    void settings.init();
    void profile.load();
    void player.start();
    void library.start();
    void downloads.start();
    let alive = true;
    let un: (() => void) | undefined;
    let revision = 0;
    void api.on('metadata://progress', (status) => {
      if (!alive) return;
      revision++;
      metadata = status;
      if (!status.running && status.finishedAt != null) {
        toasts.push(status.cancelled ? `Reprocessamento parado em ${status.done} de ${status.total} faixas.` : `Metadados reprocessados: ${status.done - status.failed} de ${status.total} faixas.`, 'info');
      }
    }).then(async (unsubscribe) => {
      if (!alive) { unsubscribe(); return; }
      un = unsubscribe;
      const before = revision;
      try {
        const status = await api.metadataRefreshStatus();
        if (alive && before === revision) metadata = status;
      } catch (e) { if (alive) notifyError(e); }
    }).catch((e) => { if (alive) notifyError(e); });
    return () => { alive = false; un?.(); window.clearTimeout(gpuTimer); };
  });

  let main: HTMLElement;
  $effect(() => { router.current; main?.scrollTo({ top: 0 }); });

  function isTyping(t: EventTarget | null) {
    const el = t as HTMLElement | null;
    if (!el) return false;
    const tag = el.tagName;
    return tag === 'INPUT' && !['range', 'checkbox', 'radio', 'button'].includes((el as HTMLInputElement).type) || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable;
  }

  // Atalhos (desktop): espaço, setas esquerda/direita (seek 5 s), cima/baixo (volume), M (mudo).
  function onKey(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented || isTyping(e.target)) return;
    const el = e.target as HTMLElement;
    const onControl = el.tagName === 'BUTTON' || el.tagName === 'A' || (el.tagName === 'INPUT' && (el as HTMLInputElement).type === 'range');
    switch (e.key) {
      case ' ':
        if (el.tagName === 'BUTTON' || el.tagName === 'A') return; // espaço ativa o controle focado
        e.preventDefault(); void player.toggle(); break;
      case 'ArrowRight': if (onControl && el.tagName === 'INPUT') return; e.preventDefault(); void player.seekBy(5000); break;
      case 'ArrowLeft': if (onControl && el.tagName === 'INPUT') return; e.preventDefault(); void player.seekBy(-5000); break;
      case 'ArrowUp': if (onControl && el.tagName === 'INPUT') return; e.preventDefault(); void player.setVolume(player.state.volume + 0.05); break;
      case 'ArrowDown': if (onControl && el.tagName === 'INPUT') return; e.preventDefault(); void player.setVolume(player.state.volume - 0.05); break;
      case 'm': case 'M': void player.toggleMuted(); break;
    }
  }
  const name = $derived(router.current.name);
  let routeKey = $state('');
  $effect.pre(() => {
    const next = `${router.current.name}:${router.current.params.id ?? ''}`;
    if (routeKey && routeKey !== next) {
      captureSharedLayout(name !== 'now-playing' && routeKey.startsWith('now-playing:'));
    }
    routeKey = next;
  });
</script>

<svelte:window onkeydown={onKey} />

<div class="flex h-dvh flex-col" style="padding-top:var(--safe-t)">
  <div class="flex min-h-0 flex-1">
    <Nav variant="side" />
    <main bind:this={main} class="relative min-w-0 flex-1 overflow-y-auto" id="conteudo" tabindex="-1">
      {#if isMock}
        <div class="border-b border-line bg-s1 px-4 py-1 text-center text-xs text-faint">Modo demonstração: dados fictícios em memória (fora do Tauri).</div>
      {/if}
      {#key `${name}:${router.current.params.id ?? ''}`}
      {#if name === 'now-playing'}
        <div transition:springTransition><NowPlaying /></div>
      {:else}
        <div class="mx-auto max-w-6xl p-4 md:p-8" transition:springTransition>
          {#if name === 'home'}<Home />
          {:else if name === 'library'}<Library />
          {:else if name === 'playlists'}<Playlists />
          {:else if name === 'playlist'}{#key router.current.params.id}<PlaylistDetail id={router.current.params.id} />{/key}
          {:else if name === 'youtube'}<YouTube />
          {:else if name === 'downloads'}<Downloads />
          {:else if name === 'artist'}{#key router.current.params.id}<Artist id={router.current.params.id} />{/key}
          {:else if name === 'stats'}<Stats />
          {:else if name === 'dna'}<Dna />
          {:else if name === 'wrapped'}<Wrapped />
          {:else if name === 'profile'}<Profile />
          {:else if name === 'settings'}<SettingsPage />
          {:else}<State kind="empty" icon="alert" title="Página não encontrada" />
          {/if}
        </div>
      {/if}
      {/key}
    </main>
  </div>

  <!-- @req RF-17: em Tocando os controles ficam na própria tela -->
  {#if name !== 'now-playing'}
    <div class="hidden md:block" transition:springCollapse><PlayerBar /></div>
  {/if}
  <div class="md:hidden">
    <MiniPlayer />
    <Nav variant="bottom" />
  </div>
</div>

<Toasts />
{#if metadata?.running && hiddenBatch !== metadata.startedAt}
  {@const status = metadata}
  <PinnedToast title={`Reprocessando metadados — ${status.done} de ${status.total}`} onclose={() => { hiddenBatch = status.startedAt; }}>
    {#snippet content()}
      <div class="mt-3 h-2 overflow-hidden rounded-full bg-s3" role="progressbar" aria-label="Andamento do reprocessamento de metadados" aria-valuemin="0" aria-valuemax={Math.max(status.total, 1)} aria-valuenow={status.done}>
        <div class="h-full bg-lava" style:width={`${status.total ? status.done / status.total * 100 : 0}%`}></div>
      </div>
      <button type="button" class="btn btn-sm mt-3" disabled={status.stopping} onclick={() => { void api.metadataRefreshCancel().catch(notifyError); }}>{status.stopping ? 'Parando…' : 'Parar'}</button>
    {/snippet}
  </PinnedToast>
{/if}
<Dialogs />
