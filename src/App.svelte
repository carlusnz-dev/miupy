<script lang="ts">
  // @req RNF-08
  // Casca da aplicação: layout responsivo, inicialização das stores, atalhos de teclado.
  // @req RNF-03 RNF-04
  import { onMount, untrack } from 'svelte';
  import { springTransition } from './lib/motion';
  import { api, isMock, type MetadataRefreshStatus } from './lib/ipc';
  import { notifyError, toasts } from './lib/stores/toast.svelte';
  import { router, navigate, paths, type Route } from './lib/stores/router.svelte';
  import { player } from './lib/stores/player.svelte';
  import { library } from './lib/stores/library.svelte';
  import { downloads } from './lib/stores/downloads.svelte';
  import { settings } from './lib/stores/settings.svelte';
  import { profile } from './lib/stores/profile.svelte';
  import { isTyping } from './lib/util/typing';
  import { beginDrag } from './lib/util/drag';
  import Nav from './lib/components/Nav.svelte';
  import PlayerBar from './lib/components/PlayerBar.svelte';
  import RightPanel from './lib/components/RightPanel.svelte';
  import MiniPlayer from './lib/components/MiniPlayer.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import PinnedToast from './lib/components/PinnedToast.svelte';
  import Dialogs from './lib/components/Dialogs.svelte';
  import State from './lib/components/State.svelte';
  import BackButton from './lib/components/BackButton.svelte';
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
  // No celular, abrir e fechar o Tocando (sobreposição) não mexe na rolagem da tela de baixo.
  let prevName = router.current.name;
  $effect(() => {
    const next = router.current.name;
    const overlayChange = untrack(() => mobile) && (next === 'now-playing' || prevName === 'now-playing');
    prevName = next;
    if (!overlayChange) main?.scrollTo({ top: 0 });
  });

  // Atalhos (desktop): espaço, setas esquerda/direita (seek 5 s), cima/baixo (volume), M (mudo).
  function onKey(e: KeyboardEvent) {
    // @req RF-43
    if (e.altKey && !e.ctrlKey && !e.metaKey && !e.repeat && e.key === 'ArrowLeft' && router.canGoBack && !isTyping(e.target)) {
      e.preventDefault(); router.back(); return;
    }
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
  // @req RF-43
  function onMouseUp(e: MouseEvent) {
    if (e.button === 3 && router.canGoBack && performance.now() - router.lastChange > 400) { e.preventDefault(); router.back(); }
  }
  const name = $derived(router.current.name);
  // @req RF-46 RNF-08
  let mobile = $state(typeof window !== 'undefined' && window.matchMedia('(max-width: 767px)').matches);
  let lastRoute = $state<Route>(router.current.name === 'now-playing' ? { name: 'home', params: {} } : router.current);
  $effect(() => { if (router.current.name !== 'now-playing') lastRoute = router.current; });
  const view = $derived(mobile && name === 'now-playing' ? lastRoute : router.current);
  const viewName = $derived(view.name);
  let overlay = $state<HTMLElement>();
  let drag: { pointerId: number; session: ReturnType<typeof beginDrag> } | null = null;
  let dragOffset = $state(0);
  let dragging = $state(false);
  let restoreFocus = $state(false);
  const reduced = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  $effect(() => {
    if (mobile && name === 'now-playing') overlay?.focus();
    else if (restoreFocus) {
      restoreFocus = false;
      (document.querySelector<HTMLButtonElement>('[aria-label^="Abrir tocando agora:"]') ?? main)?.focus();
    }
  });
  onMount(() => {
    const query = window.matchMedia('(max-width: 767px)');
    const sync = () => { mobile = query.matches; };
    query.addEventListener('change', sync);
    return () => query.removeEventListener('change', sync);
  });
  function closeNowPlaying() {
    restoreFocus = true;
    if (router.canGoBack) router.back();
    else navigate(paths.home);
  }
  function dragStart(event: PointerEvent) {
    if (event.button !== 0 || !(event.target as Element).closest('[data-drag-handle]') || (event.target as Element).closest('button')) return;
    drag = { pointerId: event.pointerId, session: beginDrag(event.clientY, event.timeStamp) };
    dragging = true;
    overlay?.setPointerCapture(event.pointerId);
  }
  function dragMove(event: PointerEvent) {
    if (drag?.pointerId !== event.pointerId) return;
    dragOffset = reduced() ? 0 : Math.max(0, drag.session.move(event.clientY, event.timeStamp));
  }
  function dragEnd(event: PointerEvent) {
    if (drag?.pointerId !== event.pointerId) return;
    const session = drag.session;
    session.move(event.clientY, event.timeStamp);
    const close = session.finish(overlay?.clientHeight ?? 0, 1, 0.3, event.timeStamp);
    drag = null; dragging = false; dragOffset = 0;
    if (close) closeNowPlaying();
  }
  function dragCancel(event: PointerEvent) {
    if (drag?.pointerId !== event.pointerId) return;
    drag.session.cancel(); drag = null; dragging = false; dragOffset = 0;
  }
</script>

<svelte:window onkeydown={onKey} onmouseup={onMouseUp} />

<div class="flex h-dvh flex-col" style="padding-top:var(--safe-t)" inert={mobile && name === 'now-playing'}>
  <!-- @req RF-38 RF-39: grade menu | conteúdo | painel da direita -->
  <div class="relative grid min-h-0 flex-1 grid-cols-[auto_minmax(0,1fr)_auto]">
    <Nav variant="side" />
    <div class="relative col-start-2 flex min-h-0 min-w-0 flex-col">
    <main bind:this={main} class="relative min-h-0 flex-1 overflow-y-auto {name !== 'now-playing' ? 'md:pb-[calc(var(--player-h)+2.5rem)]' : ''}" id="conteudo" tabindex="-1">
      {#if isMock}
        <div class="border-b border-line bg-s1 px-4 py-1 text-center text-xs text-faint">Modo demonstração: dados fictícios em memória (fora do Tauri).</div>
      {/if}
      {#key `${viewName}:${view.params.id ?? ''}`}
      {#if router.fromLink && !(mobile && name === 'now-playing')}
        {#if viewName === 'now-playing'}<div class="absolute left-4 top-4 z-20 md:left-8 md:top-6"><BackButton /></div>
        {:else}<div class="mx-auto max-w-6xl px-4 pt-4 md:px-8 md:pt-6"><BackButton /></div>{/if}
      {/if}
      {#if viewName === 'now-playing' && !mobile}
        <div transition:springTransition><NowPlaying /></div>
      {:else}
        {#if viewName !== 'now-playing'}<div class="mx-auto max-w-6xl p-4 md:p-8" transition:springTransition>
          {#if viewName === 'home'}<Home />
          {:else if viewName === 'library'}<Library />
          {:else if viewName === 'playlists'}<Playlists />
          {:else if viewName === 'playlist'}{#key view.params.id}<PlaylistDetail id={view.params.id} />{/key}
          {:else if viewName === 'youtube'}<YouTube />
          {:else if viewName === 'downloads'}<Downloads />
          {:else if viewName === 'artist'}{#key view.params.id}<Artist id={view.params.id} />{/key}
          {:else if viewName === 'stats'}<Stats />
          {:else if viewName === 'dna'}<Dna />
          {:else if viewName === 'wrapped'}<Wrapped />
          {:else if viewName === 'profile'}<Profile />
          {:else if viewName === 'settings'}<SettingsPage />
          {:else}<State kind="empty" icon="alert" title="Página não encontrada" />
          {/if}
        </div>{/if}
      {/if}
      {/key}
    </main>
    <!-- @req RF-17: em Tocando os controles ficam na própria tela -->
    {#if name !== 'now-playing'}
      <div class="pointer-events-none absolute inset-x-5 bottom-4 hidden md:block" transition:springTransition><PlayerBar /></div>
    {/if}
    </div>
    <!-- @req RF-39: em Tocando a tela já mostra fila e letra -->
    {#if name !== 'now-playing'}<RightPanel />{/if}
  </div>

  <div class="md:hidden">
    {#if name !== 'now-playing'}<MiniPlayer />{/if}
    <Nav variant="bottom" />
  </div>
</div>

{#if mobile && name === 'now-playing'}
  <!-- @req RF-46: a tela cobre inclusive as abas e o mini player. -->
  <div bind:this={overlay} class="fixed inset-0 z-[60] overflow-y-auto" style="background:var(--bg);padding-top:var(--safe-t);padding-bottom:var(--safe-b);transform:translate3d(0,{dragOffset}px,0);transition:{dragging || reduced() ? 'none' : 'transform var(--dur-base) var(--ease-spring)'}"
    role="dialog" aria-modal="true" aria-label="Tocando agora" tabindex="-1"
    transition:springTransition={{ y: typeof window !== 'undefined' ? window.innerHeight : 800, duration: 420, reducedDuration: 0 }}
    onpointerdown={dragStart} onpointermove={dragMove} onpointerup={dragEnd} onpointercancel={dragCancel} onlostpointercapture={dragCancel}
    onkeydown={(event) => { if (event.key === 'Escape') { event.preventDefault(); closeNowPlaying(); } }}>
    <NowPlaying mobile onclose={closeNowPlaying} />
  </div>
{/if}

<Toasts />
{#if metadata?.running && hiddenBatch !== metadata.startedAt}
  {@const status = metadata}
  <PinnedToast title="Reprocessando metadados" onclose={() => { hiddenBatch = status.startedAt; }}>
    {#snippet content()}
      <p class="mt-1 text-xs text-muted">{status.done} de {status.total} faixas</p>
      <div class="mt-3 h-2 overflow-hidden rounded-full bg-s3" role="progressbar" aria-label="Andamento do reprocessamento de metadados" aria-valuemin="0" aria-valuemax={Math.max(status.total, 1)} aria-valuenow={status.done}>
        <div class="h-full bg-lava" style:width={`${status.total ? status.done / status.total * 100 : 0}%`}></div>
      </div>
      <button type="button" class="btn btn-sm mt-3" disabled={status.stopping} onclick={() => { void api.metadataRefreshCancel().catch(notifyError); }}>{status.stopping ? 'Parando…' : 'Parar'}</button>
    {/snippet}
  </PinnedToast>
{/if}
<Dialogs />
