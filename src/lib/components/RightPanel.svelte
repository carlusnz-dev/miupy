<script lang="ts">
  // @req RF-39
  // Painel recolhível à direita: faixa atual e fila.
  import { player } from '../stores/player.svelte';
  import { settings } from '../stores/settings.svelte';
  import { favorites } from '../stores/favorites.svelte';
  import { href, paths } from '../stores/router.svelte';
  import { toggleFavorite } from '../actions';
  import { enterStyle } from '../motion';
  import { Chunked } from '../util/chunked.svelte';
  import { fmtTime } from '../util/format';
  import { isTyping } from '../util/typing';
  import { rightPanelOpen, upcoming } from '../util/rightPanel';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';

  let width = $state(typeof window === 'undefined' ? 0 : window.innerWidth);
  const open = $derived(rightPanelOpen(settings.rightPanel, width));
  const t = $derived(player.track);
  const toggle = () => settings.setRightPanel(!open);
  // Ctrl+J; o Ctrl+B é do menu lateral.
  function onShortcut(e: KeyboardEvent) {
    if (e.defaultPrevented || isTyping(e.target) || !e.ctrlKey || e.metaKey || e.altKey || e.shiftKey || e.key.toLowerCase() !== 'j') return;
    e.preventDefault();
    toggle();
  }

  const rows = $derived(upcoming(player.queueTracks, player.state.queueIndex));
  const chunk = new Chunked();
  $effect(() => chunk.advance(rows.length));
</script>

<svelte:window bind:innerWidth={width} onkeydown={onShortcut} />

<div class="right-shell absolute inset-y-0 right-0 z-20 hidden xl:static md:block" class:open>
  <button type="button" class="right-toggle" onclick={toggle} aria-expanded={open} aria-controls="painel-direito"
          aria-label={open ? 'Recolher painel da direita' : 'Expandir painel da direita'} title={open ? 'Recolher painel (Ctrl+J)' : 'Expandir painel (Ctrl+J)'}>
    <Icon name={open ? 'chevron-right' : 'chevron-left'} size={14} />
  </button>
  <div class="h-full overflow-hidden">
  {#if open}
    <aside id="painel-direito" class="right-panel flex h-full flex-col gap-4 overflow-y-auto border-l border-line bg-s1 p-4" aria-label="Painel da direita">
      {#if t}
        <section class="enter flex flex-col gap-3" aria-label="Faixa atual">
          <a href={href(paths.nowPlaying)} aria-label="Abrir tocando agora" class="block"><Cover path={t.coverPath} thumb={t.thumbnailUrl} seed={t.id} size={288} rounded="0.5rem" /></a>
          <div class="flex items-center gap-2">
            <div class="min-w-0 flex-1">
              <p class="truncate font-semibold">{t.title}</p>
              {#if t.artistId}
                <a class="block truncate text-sm text-muted hover:underline" href={href(paths.artist(t.artistId))}>{t.artistName}</a>
              {:else}<p class="truncate text-sm text-muted">{t.artistName ?? 'Artista desconhecido'}</p>{/if}
            </div>
            <button class="icon-btn motion-bounce {favorites.is(t) ? 'on' : ''}" aria-pressed={favorites.is(t)} aria-label={favorites.is(t) ? 'Remover das favoritas' : 'Favoritar'} onclick={() => toggleFavorite(t)}>
              <Icon name="heart" size={18} fill={favorites.is(t)} />
            </button>
          </div>
        </section>
      {:else}
        <p class="enter text-sm text-faint">Nada tocando</p>
      {/if}
      <section class="flex flex-col gap-1" aria-label="Fila">
        <h2 class="enter px-1 text-sm font-semibold" style="--enter-delay:80ms">Fila</h2>
        {#if rows.length === 0}
          <p class="enter px-1 text-sm text-faint" style="--enter-delay:120ms">Fila vazia</p>
        {:else}
          <ol>
            {#each rows.slice(0, chunk.shown) as { track: q, at }, i (q.id + at)}
              {@const current = at === player.state.queueIndex}
              <li class="enter" style={enterStyle(i)}>
                <button class="flex w-full items-center gap-3 rounded-lg p-2 text-left hover:bg-s2 {current ? 'bg-s2' : ''}" aria-current={current ? 'true' : undefined} onclick={() => player.play(q.id, player.state.queue)}>
                  <Cover path={q.coverPath} thumb={q.thumbnailUrl} seed={q.id} size={36} rounded="0.25rem" />
                  <span class="min-w-0 flex-1">
                    <span class="block truncate text-sm {current ? 'text-lava-text' : ''}">{q.title}</span>
                    <span class="block truncate text-xs text-muted">{q.artistName}</span>
                  </span>
                  <span class="tnum text-xs text-muted">{fmtTime(q.durationMs)}</span>
                </button>
              </li>
            {/each}
          </ol>
          {#if chunk.shown < rows.length}<div aria-hidden="true" style="height:{chunk.rest(rows.length)}px"></div>{/if}
        {/if}
      </section>
    </aside>
  {/if}
  </div>
</div>
