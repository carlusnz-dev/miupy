<script lang="ts">
  // Casca da aplicação: layout responsivo, inicialização das stores, atalhos de teclado.
  // @req RNF-03 RNF-04
  import { onMount } from 'svelte';
  import { isMock } from './lib/ipc';
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
  import Wrapped from './routes/Wrapped.svelte';
  import Profile from './routes/Profile.svelte';
  import SettingsPage from './routes/Settings.svelte';

  onMount(() => {
    void settings.init();
    void profile.load();
    void player.start();
    void library.start();
    void downloads.start();
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
</script>

<svelte:window onkeydown={onKey} />

<div class="flex h-dvh flex-col" style="padding-top:var(--safe-t)">
  <div class="flex min-h-0 flex-1">
    <Nav variant="side" />
    <main bind:this={main} class="relative min-w-0 flex-1 overflow-y-auto" id="conteudo" tabindex="-1">
      {#if isMock}
        <div class="border-b border-line bg-s1 px-4 py-1 text-center text-xs text-faint">Modo demonstração: dados fictícios em memória (fora do Tauri).</div>
      {/if}
      {#if name === 'now-playing'}
        <NowPlaying />
      {:else}
        <div class="mx-auto max-w-6xl p-4 md:p-8">
          {#if name === 'home'}<Home />
          {:else if name === 'library'}<Library />
          {:else if name === 'playlists'}<Playlists />
          {:else if name === 'playlist'}{#key router.current.params.id}<PlaylistDetail id={router.current.params.id} />{/key}
          {:else if name === 'youtube'}<YouTube />
          {:else if name === 'downloads'}<Downloads />
          {:else if name === 'artist'}{#key router.current.params.id}<Artist id={router.current.params.id} />{/key}
          {:else if name === 'stats'}<Stats />
          {:else if name === 'wrapped'}<Wrapped />
          {:else if name === 'profile'}<Profile />
          {:else if name === 'settings'}<SettingsPage />
          {:else}<State kind="empty" icon="alert" title="Página não encontrada" />
          {/if}
        </div>
      {/if}
    </main>
  </div>

  <div class="hidden md:block"><PlayerBar /></div>
  <div class="md:hidden">
    <MiniPlayer />
    <Nav variant="bottom" />
  </div>
</div>

<Toasts />
<Dialogs />
