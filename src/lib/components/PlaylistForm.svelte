<script lang="ts">
  // Criar e editar detalhes da playlist (nome, descrição e capa) em um modal. @req RF-06 RF-22
  import { api, errorMessage, type ImageUpload, type Playlist } from '../ipc';
  import { library } from '../stores/library.svelte';
  import { notify } from '../stores/toast.svelte';
  import { previewImage, readImageUpload } from '../util/imageUpload';
  import Cover from './Cover.svelte';
  import Modal from './Modal.svelte';
  import PlaylistCover from './PlaylistCover.svelte';

  type Props = { state: { playlist: Playlist | null } | null; onclose: () => void; ondone?: (p: Playlist) => void };
  let { state: st, onclose, ondone }: Props = $props();

  let name = $state('');
  let description = $state('');
  let err = $state('');
  let coverErr = $state('');
  let busy = $state(false);
  // undefined: capa intacta; null: remover; imagem: trocar. Só vale ao salvar.
  let cover = $state<ImageUpload | null | undefined>(undefined);
  let coverInput = $state<HTMLInputElement>();
  const hasCover = $derived(cover === undefined ? !!st?.playlist?.coverPath : cover !== null);
  $effect(() => { if (st) { name = st.playlist?.name ?? ''; description = st.playlist?.description ?? ''; err = ''; coverErr = ''; cover = undefined; } });

  async function pickCover(file: File | undefined) {
    if (!file) return;
    try { cover = await readImageUpload(file); coverErr = ''; } catch (ex) { coverErr = errorMessage(ex); }
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) { err = 'O nome não pode ficar vazio.'; return; }
    busy = true; err = '';
    try {
      let p = st?.playlist
        ? await api.playlistUpdate({ playlistId: st.playlist.id, name, description })
        : await api.playlistCreate({ name, description: description || undefined });
      if (st?.playlist && cover !== undefined) {
        p = await api.playlistSetCover({ playlistId: p.id, image: cover });
        await library.reloadPlaylists();
      }
      notify(st?.playlist ? 'Playlist atualizada.' : 'Playlist criada.');
      ondone?.(p);
      onclose();
    } catch (ex) { err = errorMessage(ex); } finally { busy = false; }
  }
</script>

<Modal open={st !== null} title={st?.playlist ? 'Editar detalhes' : 'Nova playlist'} {onclose}>
  <form id="pl-form" class="flex flex-col gap-3" onsubmit={submit} novalidate>
    {#if st?.playlist}
      <div class="flex items-center gap-4">
        {#if cover}
          <Cover url={previewImage(cover)} seed={st.playlist.id} size={96} rounded="0.75rem" label="Nova capa" />
        {:else}
          <PlaylistCover playlist={cover === null ? { ...st.playlist, coverPath: null } : st.playlist} size={96} />
        {/if}
        <div class="min-w-0">
          <div class="flex flex-wrap gap-2">
            <input bind:this={coverInput} type="file" accept="image/jpeg,image/png,image/webp" class="sr-only" tabindex="-1" aria-label="Arquivo da capa" onchange={(e) => { void pickCover(e.currentTarget.files?.[0]); e.currentTarget.value = ''; }} />
            <button type="button" class="btn btn-sm" onclick={() => coverInput?.click()}>Escolher capa</button>
            {#if hasCover}<button type="button" class="btn btn-sm" onclick={() => (cover = null)}>Remover capa</button>{/if}
          </div>
          {#if coverErr}<p class="mt-1 text-sm text-danger" role="alert">{coverErr}</p>{/if}
        </div>
      </div>
    {/if}
    <div>
      <label class="mb-1 block text-sm text-muted" for="pl-name">Nome</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="pl-name" class="field" bind:value={name} maxlength="80" autofocus aria-invalid={err ? 'true' : undefined} aria-describedby={err ? 'pl-err' : undefined} />
      {#if err}<p id="pl-err" class="mt-1 text-sm text-danger" role="alert">{err}</p>{/if}
    </div>
    <div>
      <label class="mb-1 block text-sm text-muted" for="pl-desc">Descrição (opcional)</label>
      <textarea id="pl-desc" class="field" rows="2" bind:value={description} maxlength="300"></textarea>
    </div>
  </form>
  {#snippet footer()}
    <button class="btn btn-sm" type="button" onclick={onclose}>Cancelar</button>
    <button class="btn btn-sm btn-primary" form="pl-form" disabled={busy}>{st?.playlist ? 'Salvar' : 'Criar'}</button>
  {/snippet}
</Modal>
