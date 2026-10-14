<script lang="ts">
  // Navegação: barra lateral (desktop) e inferior (mobile). @req RNF-03
  import { router, href, paths, type RouteName } from '../stores/router.svelte';
  import { downloads } from '../stores/downloads.svelte';
  import { profile } from '../stores/profile.svelte';
  import Icon from './Icon.svelte';

  let { variant }: { variant: 'side' | 'bottom' } = $props();

  type Item = { label: string; icon: string; path: string; names: RouteName[] };
  const main: Item[] = [
    { label: 'Início', icon: 'home', path: paths.home, names: ['home'] },
    { label: 'Biblioteca', icon: 'library', path: paths.library, names: ['library', 'artist'] },
    { label: 'Playlists', icon: 'playlist', path: paths.playlists, names: ['playlists', 'playlist'] },
    { label: 'Buscar no YouTube', icon: 'search', path: paths.youtube, names: ['youtube'] },
    { label: 'Downloads', icon: 'download', path: paths.downloads, names: ['downloads'] },
  ];
  const insights: Item[] = [
    { label: 'Estatísticas', icon: 'stats', path: paths.stats, names: ['stats'] },
    { label: 'Retrospectiva', icon: 'sparkle', path: paths.wrapped, names: ['wrapped'] },
  ];
  const account: Item[] = [
    { label: 'Perfil', icon: 'user', path: paths.profile, names: ['profile'] },
    { label: 'Configurações', icon: 'settings', path: paths.settings, names: ['settings'] },
  ];
  const tabs: Item[] = [
    { label: 'Início', icon: 'home', path: paths.home, names: ['home'] },
    { label: 'Biblioteca', icon: 'library', path: paths.library, names: ['library', 'artist'] },
    { label: 'Playlists', icon: 'playlist', path: paths.playlists, names: ['playlists', 'playlist'] },
    { label: 'Buscar', icon: 'search', path: paths.youtube, names: ['youtube'] },
  ];
  const moreNames: RouteName[] = ['downloads', 'stats', 'wrapped', 'profile', 'settings'];
  const moreItems = [main[4], ...insights, ...account];

  let moreOpen = $state(false);
  const isOn = (i: Item) => i.names.includes(router.current.name);
  $effect(() => { router.current; moreOpen = false; });
</script>

{#snippet link(i: Item)}
  <a href={href(i.path)} aria-current={isOn(i) ? 'page' : undefined}
     class="flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium text-muted hover:bg-s2 hover:text-fg aria-[current=page]:bg-s2 aria-[current=page]:text-fg">
    <Icon name={i.icon} size={20} class={isOn(i) ? 'text-lava-text' : ''} />
    <span class="flex-1">{i.label}</span>
    {#if i.path === paths.downloads && downloads.activeCount > 0}
      <span class="tnum rounded-full px-1.5 text-xs" style="background:var(--grad-lava);color:var(--lava-ink)">{downloads.activeCount}</span>
    {/if}
  </a>
{/snippet}

{#if variant === 'side'}
<nav class="hidden h-full w-[var(--sidebar-w)] shrink-0 flex-col gap-1 overflow-y-auto border-r border-line bg-s1 p-3 md:flex" aria-label="Principal">
  <a href={href(paths.home)} class="mb-3 flex items-center gap-2 px-3 py-2" aria-label="miupy, início">
    <span class="grid h-8 w-8 place-items-center rounded-lg text-lava-ink" style="background:var(--grad-lava)"><Icon name="flame" size={18} fill /></span>
    <span class="text-lg font-bold tracking-tight">miupy</span>
  </a>
  {#each main as i}{@render link(i)}{/each}
  <p class="eyebrow mt-4 px-3 pb-1">Discoteca</p>
  {#each insights as i}{@render link(i)}{/each}
  <div class="mt-auto border-t border-line pt-3">
    {#each account as i}{@render link(i)}{/each}
    {#if profile.data}
      <p class="truncate px-3 pt-2 text-xs text-faint">{profile.data.displayName} · @{profile.data.username}</p>
    {/if}
  </div>
</nav>
{:else}
<nav class="relative border-t border-line bg-s1 md:hidden" style="padding-bottom:var(--safe-b)" aria-label="Principal">
  {#if moreOpen}
    <div class="absolute bottom-full right-2 mb-2 w-60 rounded-xl border border-line-strong bg-s2 p-1.5">
      {#each moreItems as i}{@render link(i)}{/each}
    </div>
  {/if}
  <ul class="grid h-[var(--nav-h)] grid-cols-5">
    {#each tabs as i}
      <li>
        <a href={href(i.path)} aria-current={isOn(i) ? 'page' : undefined}
           class="flex h-full flex-col items-center justify-center gap-0.5 text-[0.6875rem] font-medium text-muted aria-[current=page]:text-lava-text">
          <Icon name={i.icon} size={22} />{i.label}
        </a>
      </li>
    {/each}
    <li>
      <button class="relative flex h-full w-full flex-col items-center justify-center gap-0.5 border-0 bg-transparent text-[0.6875rem] font-medium {moreNames.includes(router.current.name) || moreOpen ? 'text-lava-text' : 'text-muted'}"
              aria-expanded={moreOpen} aria-haspopup="true" onclick={() => (moreOpen = !moreOpen)}>
        <Icon name="more" size={22} />Mais
        {#if downloads.activeCount > 0}<span class="absolute right-4 top-2 h-2 w-2 rounded-full" style="background:var(--lava)"></span>{/if}
      </button>
    </li>
  </ul>
</nav>
{/if}
