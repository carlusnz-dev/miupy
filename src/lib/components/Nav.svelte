<script lang="ts">
  // Navegação: barra lateral (desktop) e inferior (mobile). @req RNF-03
  import { router, href, paths, type RouteName } from '../stores/router.svelte';
  import { downloads } from '../stores/downloads.svelte';
  import { profile } from '../stores/profile.svelte';
  import Icon from './Icon.svelte';
  import Avatar from './Avatar.svelte';
  import { settings } from '../stores/settings.svelte';
  import { notifyError } from '../stores/toast.svelte';
  import { isTyping } from '../util/typing';
  import { beginSidebarDrag, clampSidebarWidth, createDeferredSave, keyboardSidebarWidth, SIDEBAR_DEFAULT } from '../util/sidebar';

  let { variant }: { variant: 'side' | 'bottom' } = $props();

  type Item = { label: string; icon: string; path: string; names: RouteName[] };
  const main: Item[] = [
    { label: 'Início', icon: 'home', path: paths.home, names: ['home'] },
    { label: 'Discoteca', icon: 'library', path: paths.library, names: ['library', 'artist'] },
    { label: 'Playlists', icon: 'playlist', path: paths.playlists, names: ['playlists', 'playlist'] },
    { label: 'Buscar no YouTube', icon: 'search', path: paths.youtube, names: ['youtube'] },
    { label: 'Downloads', icon: 'download', path: paths.downloads, names: ['downloads'] },
  ];
  const insights: Item[] = [
    { label: 'Estatísticas', icon: 'stats', path: paths.stats, names: ['stats'] },
    { label: 'DNA', icon: 'sparkle', path: paths.dna, names: ['dna'] },
    { label: 'Retrospectiva', icon: 'sparkle', path: paths.wrapped, names: ['wrapped'] },
  ];
  const account: Item[] = [
    { label: 'Perfil', icon: 'user', path: paths.profile, names: ['profile'] },
    { label: 'Configurações', icon: 'settings', path: paths.settings, names: ['settings'] },
  ];
  const tabs: Item[] = [
    { label: 'Início', icon: 'home', path: paths.home, names: ['home'] },
    { label: 'Discoteca', icon: 'library', path: paths.library, names: ['library', 'artist'] },
    { label: 'Playlists', icon: 'playlist', path: paths.playlists, names: ['playlists', 'playlist'] },
    { label: 'Buscar', icon: 'search', path: paths.youtube, names: ['youtube'] },
  ];
  const moreNames: RouteName[] = ['downloads', 'stats', 'dna', 'wrapped', 'profile', 'settings'];
  const moreItems = [main[4], ...insights, ...account];

  // @req RF-38
  const collapsed = $derived(settings.sidebarCollapsed);
  const toggleCollapsed = () => { keySave.flush(); settings.setSidebarCollapsed(!collapsed); };
  function onShortcut(e: KeyboardEvent) {
    if (variant !== 'side' || e.defaultPrevented || isTyping(e.target) || !e.ctrlKey || e.metaKey || e.altKey || e.shiftKey || e.key.toLowerCase() !== 'b') return;
    e.preventDefault();
    toggleCollapsed();
  }

  let moreOpen = $state(false);
  const isOn = (i: Item) => i.names.includes(router.current.name);
  // @req RF-43
  function menuClick(event: MouseEvent, path: string) {
    if (event.button !== 0 || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) return;
    event.preventDefault();
    router.menuNavigate(path);
  }
  $effect(() => { router.current; moreOpen = false; });

  // @req RF-33
  let resizeHandle = $state<HTMLElement>();
  let drag: { pointerId: number; session: ReturnType<typeof beginSidebarDrag> } | null = null;
  const currentWidth = () => clampSidebarWidth(parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--sidebar-w')));
  function previewWidth(width: number) {
    document.documentElement.style.setProperty('--sidebar-w', `${width}px`);
    resizeHandle?.setAttribute('aria-valuenow', String(width));
  }
  function persistWidth(width: number) {
    void settings.saveSidebarWidth(width).catch((error) => {
      previewWidth(settings.sidebarWidth);
      notifyError(error);
    });
  }
  // teclas só pré-visualizam; a largura é gravada uma vez, quando o ajuste termina
  const keySave = createDeferredSave((width) => { if (width !== settings.sidebarWidth) persistWidth(width); });
  $effect(() => () => keySave.flush());
  function startResize(event: PointerEvent) {
    if (event.button !== 0) return;
    keySave.cancel();
    const width = currentWidth();
    drag = { pointerId: event.pointerId, session: beginSidebarDrag(width, event.clientX, previewWidth, persistWidth) };
    resizeHandle?.setPointerCapture(event.pointerId);
    event.preventDefault();
  }
  function moveResize(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointerId) return;
    drag.session.move(event.clientX);
  }
  function endResize(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointerId) return;
    const session = drag.session;
    drag = null;
    if (resizeHandle?.hasPointerCapture(event.pointerId)) resizeHandle.releasePointerCapture(event.pointerId);
    session.finish();
  }
  function cancelResize(event: PointerEvent) {
    if (!drag || event.pointerId !== drag.pointerId) return;
    const session = drag.session;
    drag = null;
    session.cancel();
  }
  function resizeKey(event: KeyboardEvent) {
    const width = keyboardSidebarWidth(currentWidth(), event.key);
    if (width === null) return;
    event.preventDefault();
    event.stopPropagation();
    previewWidth(width);
    keySave.schedule(width);
  }
  function resetWidth() {
    keySave.cancel();
    previewWidth(SIDEBAR_DEFAULT);
    if (settings.sidebarWidth !== SIDEBAR_DEFAULT) persistWidth(SIDEBAR_DEFAULT);
  }
</script>

{#snippet link(i: Item, rail = false)}
  <a href={href(i.path)} onclick={(event) => menuClick(event, i.path)} aria-current={isOn(i) ? 'page' : undefined} aria-label={rail ? (i.path === paths.downloads && downloads.activeCount > 0 ? `${i.label}, ${downloads.activeCount} em andamento` : i.label) : undefined} title={rail ? i.label : undefined}
     class="relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium whitespace-nowrap text-muted hover:bg-s2 hover:text-fg aria-[current=page]:bg-s2 aria-[current=page]:text-fg">
    <Icon name={i.icon} size={20} class="shrink-0 {isOn(i) ? 'text-lava-text' : ''}" />
    {#if !rail}<span class="enter flex-1">{i.label}</span>{/if}
    {#if i.path === paths.downloads && downloads.activeCount > 0}
      {#if rail}
        <span class="absolute right-1.5 top-1.5 h-2 w-2 rounded-full" style="background:var(--lava)"></span>
      {:else}
        <span class="tnum rounded-full px-1.5 text-xs" style="background:var(--grad-lava);color:var(--lava-ink)">{downloads.activeCount}</span>
      {/if}
    {/if}
  </a>
{/snippet}

<svelte:window onkeydown={onShortcut} />

{#if variant === 'side'}
<div class="nav-shell relative hidden h-full shrink-0 md:block" class:collapsed>
<nav id="nav-principal" class="nav-glass flex h-full flex-col gap-1 overflow-x-hidden overflow-y-auto border-r border-line p-3" aria-label="Principal">
  <a href={href(paths.home)} onclick={(event) => menuClick(event, paths.home)} class="mb-3 flex items-center gap-2 px-3 py-2" aria-label="miupy, início" title={collapsed ? 'miupy' : undefined}>
    <span class="grid h-8 w-8 shrink-0 place-items-center rounded-lg text-lava-ink" style="background:var(--grad-lava)"><Icon name="flame" size={18} fill /></span>
    {#if !collapsed}<span class="enter text-lg font-bold tracking-tight">miupy</span>{/if}
  </a>
  {#each main as i}{@render link(i, collapsed)}{/each}
  {#if collapsed}<hr class="mx-3 my-2 border-line" />{:else}<p class="eyebrow enter mt-4 px-3 pb-1">Discoteca</p>{/if}
  {#each insights as i}{@render link(i, collapsed)}{/each}
  <div class="mt-auto border-t border-line pt-3">
    {#each account as i}{@render link(i, collapsed)}{/each}
    {#if profile.data}
      <div class="flex items-center gap-2 px-3 pt-2"><Avatar path={profile.data.avatarPath} initials={profile.initials} size={28} />{#if !collapsed}<p class="enter truncate text-xs text-faint">{profile.data.displayName} · @{profile.data.username}</p>{/if}</div>
    {/if}
  </div>
</nav>
<button type="button" class="nav-toggle" onclick={toggleCollapsed} aria-expanded={!collapsed} aria-controls="nav-principal"
        aria-label={collapsed ? 'Expandir menu lateral' : 'Recolher menu lateral'} title={collapsed ? 'Expandir menu (Ctrl+B)' : 'Recolher menu (Ctrl+B)'}>
  <Icon name={collapsed ? 'chevron-right' : 'chevron-left'} size={14} />
</button>
<!-- Separador ajustável segundo ARIA; Svelte classifica todo separator como não interativo. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
{#if !collapsed}
<div bind:this={resizeHandle} class="sidebar-resize" role="separator" aria-label="Redimensionar menu lateral" aria-orientation="vertical"
     aria-valuemin="220" aria-valuemax="400" aria-valuenow={settings.sidebarWidth} tabindex="0"
     onpointerdown={startResize} onpointermove={moveResize} onpointerup={endResize}
     onpointercancel={cancelResize} onlostpointercapture={cancelResize}
     onkeydown={resizeKey} onblur={() => keySave.flush()} ondblclick={resetWidth}></div>
{/if}
</div>
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
        <a href={href(i.path)} onclick={(event) => menuClick(event, i.path)} aria-current={isOn(i) ? 'page' : undefined}
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
