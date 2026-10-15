<script lang="ts">
  // @req RF-39
  // Painel recolhível à direita: faixa atual e fila.
  import { player } from '../stores/player.svelte';
  import { settings } from '../stores/settings.svelte';
  import { favorites } from '../stores/favorites.svelte';
  import { href, paths } from '../stores/router.svelte';
  import { toggleFavorite } from '../actions';
  import { rightPanelOpen } from '../util/rightPanel';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';

  let width = $state(typeof window === 'undefined' ? 0 : window.innerWidth);
  const open = $derived(rightPanelOpen(settings.rightPanel, width));
  const t = $derived(player.track);
</script>

<svelte:window bind:innerWidth={width} />

<div class="right-shell absolute inset-y-0 right-0 z-20 hidden xl:static md:block" class:open>
  {#if open}
    <aside class="right-panel flex h-full flex-col gap-4 overflow-y-auto border-l border-line bg-s1 p-4" aria-label="Painel da direita">
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
    </aside>
  {/if}
</div>
