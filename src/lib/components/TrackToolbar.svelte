<script lang="ts">
  // Busca, ordenação e filtro de favoritas. @req RF-07 RF-08
  import type { SortDir, TrackSort } from '../ipc';
  import Dropdown from './Dropdown.svelte';
  import Icon from './Icon.svelte';

  type Props = {
    search: string; sortBy: TrackSort; sortDir: SortDir; favoritesOnly: boolean;
    allowPosition?: boolean; placeholder?: string;
  };
  let { search = $bindable(), sortBy = $bindable(), sortDir = $bindable(), favoritesOnly = $bindable(), allowPosition = false, placeholder = 'Buscar por título, artista ou álbum' }: Props = $props();

  const sortOptions = $derived<{ value: TrackSort; label: string }[]>([
    ...(allowPosition ? [{ value: 'position' as const, label: 'Ordem da playlist' }] : []),
    { value: 'addedAt', label: 'Adicionadas' }, { value: 'title', label: 'Título' }, { value: 'artist', label: 'Artista' },
    { value: 'album', label: 'Álbum' }, { value: 'duration', label: 'Duração' },
  ]);
</script>

<div class="mb-3 flex flex-wrap items-center gap-2">
  <div class="relative min-w-48 flex-1">
    <Icon name="search" size={16} class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-faint" />
    <input type="search" class="field !pl-9" {placeholder} aria-label="Buscar faixas" bind:value={search} />
  </div>
  <label class="sr-only" for="sort-by">Ordenar por</label>
  <Dropdown id="sort-by" bind:value={sortBy} options={sortOptions} />
  <button class="btn btn-sm !min-h-10" aria-label={sortDir === 'asc' ? 'Ordem crescente' : 'Ordem decrescente'} disabled={sortBy === 'position'} onclick={() => (sortDir = sortDir === 'asc' ? 'desc' : 'asc')}>
    <Icon name={sortDir === 'asc' ? 'up' : 'down'} size={16} />{sortDir === 'asc' ? 'Cresc.' : 'Decresc.'}
  </button>
  <button class="btn btn-sm !min-h-10 {favoritesOnly ? '!border-lava-text !text-lava-text' : ''}" aria-pressed={favoritesOnly} onclick={() => (favoritesOnly = !favoritesOnly)}>
    <Icon name="heart" size={16} fill={favoritesOnly} />Favoritas
  </button>
</div>
