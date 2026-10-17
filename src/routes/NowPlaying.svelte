<script lang="ts">
  // @req RF-18 RF-20
  // Tela cheia "tocando agora": capa, metadados, controles, seek, volume, favorito, fila e letra.
  // @req RF-01 RF-02 RF-03 RF-04 RF-05 RF-12
  import { api, type EqPreset, type Lyrics } from '../lib/ipc';
  import { settings } from '../lib/stores/settings.svelte';
  import { notifyError } from '../lib/stores/toast.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { href, paths, router } from '../lib/stores/router.svelte';
  import { toggleFavorite } from '../lib/actions';
  import { favorites } from '../lib/stores/favorites.svelte';
  import { fmtTime, parseLrc } from '../lib/util/format';
  import MeshGradient from '../lib/components/MeshGradient.svelte';
  import AnimatedText from '../lib/components/AnimatedText.svelte';
  import Cover from '../lib/components/Cover.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import Transport from '../lib/components/Transport.svelte';
  import Seek from '../lib/components/Seek.svelte';
  import Volume from '../lib/components/Volume.svelte';
  import ShuffleButton from '../lib/components/ShuffleButton.svelte';
  import { scores, fmtScore, scoreTooltip } from '../lib/stores/scores.svelte';
  import State from '../lib/components/State.svelte';
  import SkeletonList from '../lib/components/SkeletonList.svelte';
  import { dismissable } from '../lib/util/dismiss';
  import { assetCrossOrigin } from '../lib/util/asset';
  import { coverPalette } from '../lib/util/palette';

  let { mobile = false, onclose = () => {} }: { mobile?: boolean; onclose?: () => void } = $props();

  const t = $derived(player.track);
  let tab = $state<'queue' | 'lyrics'>('queue');
  let eqOpen = $state(false);
  let eqButton = $state<HTMLButtonElement>();
  let eqPresets = $state<EqPreset[]>([]);
  $effect(() => {
    void api.equalizerListPresets().then((list) => { eqPresets = list; }).catch(() => {});
  });

  function closeEq(refocus: boolean) {
    eqOpen = false;
    if (refocus) eqButton?.focus();
  }

  async function openEq() {
    if (eqOpen) { closeEq(true); return; }
    try {
      eqPresets = await api.equalizerListPresets();
      eqOpen = true;
    } catch (error) {
      notifyError(error);
    }
  }

  async function chooseEq(id: string) {
    try {
      await settings.setEqualizer(id);
      closeEq(true);
    } catch (error) {
      notifyError(error);
    }
  }

  // Cores da capa para o mesh; sem capa ou com canvas "tainted" fica `undefined` e vale o tema.
  let meshColors = $state<string[] | undefined>();
  $effect(() => {
    const src = api.resolveAsset(t?.coverPath ?? null) ?? t?.thumbnailUrl;
    if (!src) { meshColors = undefined; return; }
    let cancelled = false;
    void coverPalette(src, assetCrossOrigin(src)).then((colors) => { if (!cancelled) meshColors = colors?.length ? [colors[0], colors[1] ?? colors[0], colors[2] ?? colors[1] ?? colors[0]] : undefined; });
    return () => { cancelled = true; };
  });

  let lyrics = $state<Lyrics | null>(null);
  let lyricsLoading = $state(false);
  let lyricsErr = $state(false);

  $effect(() => {
    const id = t?.id;
    lyrics = null; lyricsErr = false;
    if (id) void scores.load([id]);
    if (!id) return;
    lyricsLoading = true;
    let cancelled = false;
    api.metadataGetLyrics({ trackId: id })
      .then((l) => { if (!cancelled) lyrics = l; })
      .catch(() => { if (!cancelled) lyricsErr = true; })
      .finally(() => { if (!cancelled) lyricsLoading = false; });
    return () => { cancelled = true; };
  });

  const lines = $derived(lyrics?.synced ? parseLrc(lyrics.synced) : []);
  const activeLine = $derived.by(() => {
    let a = -1;
    for (let i = 0; i < lines.length; i++) if (lines[i].timeMs <= player.state.positionMs) a = i; else break;
    return a;
  });
  let lyricBox = $state<HTMLElement>();
  $effect(() => {
    const i = activeLine;
    if (tab !== 'lyrics' || i < 0 || !lyricBox) return;
    const el = lyricBox.querySelector<HTMLElement>(`[data-i="${i}"]`);
    if (el) lyricBox.scrollTo({ top: el.offsetTop - lyricBox.clientHeight / 2 + el.clientHeight / 2, behavior: 'smooth' });
  });
</script>

{#if !t}
  {#if mobile}<button type="button" class="icon-btn m-3 !h-12 !w-12" aria-label="Recolher Tocando" onclick={onclose}><Icon name="down" size={24} /></button>{/if}
  <State kind="empty" icon="music" title="Nada tocando" text="Escolha uma faixa na discoteca ou numa playlist para começar.">
    <a class="btn btn-primary" href={href(paths.library)}>Ir para a discoteca</a>
  </State>
{:else}
  <!-- @req RF-40 RNF-05 RNF-13 -->
  <MeshGradient class="min-h-full" colors={meshColors} animated={router.current.name === 'now-playing'}>
    {#if mobile}
      <!-- @req RF-46: a alça inteira aceita o arrasto; o botão também funciona por teclado. -->
      <div data-drag-handle class="relative flex min-h-14 items-center justify-center touch-none">
        <span class="h-1.5 w-10 rounded-full bg-s3" aria-hidden="true"></span>
        <button type="button" class="absolute right-3 top-1 icon-btn !h-12 !w-12" aria-label="Recolher Tocando" onclick={onclose}><Icon name="down" size={24} /></button>
      </div>
    {/if}
    <div class="mx-auto grid max-w-5xl gap-6 p-4 md:grid-cols-[minmax(0,22rem)_1fr] md:gap-10 md:p-8">
      <section class="order-1 flex flex-col gap-5" aria-label="Faixa atual">
        <div data-drag-handle={mobile ? '' : undefined} class="enter flex w-full max-w-[22rem] self-center" class:touch-none={mobile}>
          <Cover path={t.coverPath} thumb={t.thumbnailUrl} seed={t.id} class="w-full" rounded="0.75rem" label="Capa de {t.title}" />
        </div>
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <h1 class="text-2xl font-bold leading-tight"><AnimatedText text={t.title} /></h1>
            <p class="enter mt-1 text-muted" style="--enter-delay:140ms">
              {#if t.artistId}
                <a class="hover:underline" href={href(paths.artist(t.artistId))}>{t.artistName}</a>
              {:else}
                <span>{t.artistName ?? 'Artista desconhecido'}</span>
              {/if}
              {#if t.album}<span class="text-faint"> · {t.album}</span>{/if}
            </p>
            <p class="enter mt-2 hidden flex-wrap gap-1.5 md:flex" style="--enter-delay:200ms">
              {#if t.codec}<span class="chip">{t.codec.toUpperCase()}{t.bitrate ? ` ${t.bitrate} kbps` : ''}</span>{/if}
              {#if player.state.streaming}<span class="chip text-lava-text">{player.isLoading ? 'Conectando…' : 'Streaming'}</span>{/if}
              {#if t.releaseYear}<span class="chip">{t.releaseYear}</span>{/if}
              {#each t.genres as genre}<span class="chip">{genre}</span>{/each}
              <span class="chip">{t.source === 'youtube' ? 'YouTube' : 'Local'}</span>
              <span class="chip">{fmtTime(t.durationMs)}</span>
              {#if scores.get(t.id)}<span class="chip" title={scoreTooltip(scores.get(t.id))}>Pontuação {fmtScore(scores.get(t.id).score)}</span>{/if}
            </p>
          </div>
        </div>
        <div class="enter relative hidden self-start md:block" style="--enter-delay:260ms" use:dismissable={(reason) => { if (eqOpen) closeEq(reason === 'escape'); }}>
          <button type="button" class="btn text-sm" aria-expanded={eqOpen} bind:this={eqButton} onclick={openEq}>
            Equalizador · {eqPresets.find((p) => p.id === settings.eqPresetId)?.name ?? 'Plano'}
          </button>
          {#if eqOpen}
            <div class="card absolute z-20 mt-1 max-h-72 min-w-44 overflow-auto p-1" aria-label="Perfis do equalizador">
              {#each eqPresets as preset}
                <button
                  type="button" class="block w-full rounded px-3 py-2 text-left text-sm hover:bg-surface"
                  aria-current={settings.eqPresetId === preset.id ? 'true' : undefined}
                  onclick={() => chooseEq(preset.id)}
                >{preset.name}</button>
              {/each}
              <button type="button" class="block w-full px-3 py-2 text-left text-sm text-muted" onclick={() => closeEq(true)}>Fechar</button>
            </div>
          {/if}
        </div>
      </section>

      <section class="order-3 min-w-0 md:order-2" aria-label="Fila e letra">
        <div class="enter mb-3 flex gap-1 rounded-full border border-line bg-s1 p-1" style="--enter-delay:120ms" role="tablist">
          <button role="tab" aria-selected={tab === 'queue'} class="btn btn-sm flex-1 !border-0 {tab === 'queue' ? '!bg-s3' : ''}" onclick={() => (tab = 'queue')}><Icon name="queue" size={16} />Fila ({player.state.queue.length})</button>
          <button role="tab" aria-selected={tab === 'lyrics'} class="btn btn-sm flex-1 !border-0 {tab === 'lyrics' ? '!bg-s3' : ''}" onclick={() => (tab = 'lyrics')}><Icon name="mic" size={16} />Letra</button>
        </div>

        <div class="enter card max-h-[60vh] overflow-y-auto bg-s1/90 p-2" style="--enter-delay:200ms" bind:this={lyricBox} role="tabpanel">
          {#if tab === 'queue'}
            {#if player.queueTracks.length === 0}
              <State kind="empty" title="Fila vazia" />
            {:else}
              <ol>
                {#each player.queueTracks as q, i (q.id + i)}
                  <li>
                    <button class="flex w-full items-center gap-3 rounded-lg p-2 text-left hover:bg-s2 {i === player.state.queueIndex ? 'bg-s2' : ''}" aria-current={i === player.state.queueIndex ? 'true' : undefined} onclick={() => player.play(q.id, player.state.queue)}>
                      <span class="tnum w-5 text-right text-xs text-faint">{i === player.state.queueIndex ? '♪' : i + 1}</span>
                      <Cover path={q.coverPath} seed={q.id} size={36} rounded="0.25rem" />
                      <span class="min-w-0 flex-1">
                        <span class="block truncate text-sm {i === player.state.queueIndex ? 'text-lava-text' : ''}">{q.title}</span>
                        <span class="block truncate text-xs text-muted">{q.artistName}</span>
                      </span>
                      <span class="tnum text-xs text-muted">{fmtTime(q.durationMs)}</span>
                    </button>
                  </li>
                {/each}
              </ol>
            {/if}
          {:else if lyricsLoading}
            <SkeletonList variant="lines" label="Buscando letra" />
          {:else if lyricsErr}
            <State kind="error" title="Não deu para buscar a letra" text="A letra vem de um serviço online; sem rede ela não aparece." />
          {:else if lines.length}
            <div class="flex flex-col gap-2 py-8 text-center text-lg font-semibold">
              {#each lines as l, i}
                <button data-i={i} class="lyric-line border-0 bg-transparent px-2 {i === activeLine ? 'lyric-on text-lava-text' : 'text-faint hover:text-muted'}" aria-current={i === activeLine ? 'true' : undefined} onclick={() => player.seek(l.timeMs)}><span data-text={l.text || '♪'}>{l.text || '♪'}</span></button>
              {/each}
            </div>
          {:else if lyrics?.plain}
            <p class="whitespace-pre-line p-4 text-center leading-relaxed text-muted">{lyrics.plain}</p>
          {:else}
            <State kind="empty" icon="mic" title="Sem letra para esta faixa" text="Não encontramos letra no serviço de metadados." />
          {/if}
        </div>
      </section>

      <section class="enter order-2 mx-auto flex w-full max-w-3xl flex-col gap-3 md:order-3 md:col-span-2" style="--enter-delay:320ms" aria-label="Controles">
        <Seek />
        <div class="flex flex-wrap items-center justify-center gap-x-4 gap-y-2 md:grid md:grid-cols-[1fr_auto_1fr]">
          <div class="flex justify-end md:justify-start"><ShuffleButton /></div>
          <Transport big />
          <div class="flex items-center justify-end gap-1">
            <button class="icon-btn motion-bounce {favorites.is(t) ? 'on' : ''}" aria-pressed={favorites.is(t)} aria-label={favorites.is(t) ? 'Remover das favoritas' : 'Favoritar'} onclick={() => toggleFavorite(t)}>
              <Icon name="heart" size={20} fill={favorites.is(t)} />
            </button>
            <button class="icon-btn {tab === 'queue' ? 'on' : ''}" aria-pressed={tab === 'queue'} aria-label="Mostrar a fila" onclick={() => (tab = 'queue')}><Icon name="queue" size={20} /></button>
            <span class="hidden md:inline-flex"><Volume /></span>
          </div>
        </div>
      </section>
    </div>
  </MeshGradient>
{/if}
