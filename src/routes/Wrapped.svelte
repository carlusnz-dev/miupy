<script lang="ts">
  // Retrospectiva anual no espírito do Wrapped: sequência de cartões. @req RF-15 RNF-05 RNF-11
  import { api, type Wrapped } from '../lib/ipc';
  import { Resource } from '../lib/stores/resource.svelte';
  import { player } from '../lib/stores/player.svelte';
  import { href, paths } from '../lib/stores/router.svelte';
  import { fmtDuration, fmtMinutes, fmtNumber, MONTHS } from '../lib/util/format';
  import MeshGradient from '../lib/components/MeshGradient.svelte';
  import Cover from '../lib/components/Cover.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import State from '../lib/components/State.svelte';
  import SkeletonList from '../lib/components/SkeletonList.svelte';

  const thisYear = new Date().getFullYear();
  // Anos e retrospectiva de cada ano ficam em cache: voltar a um ano já visto mostra na hora.
  const yearsRes = new Resource<number[]>([]);
  yearsRes.prime('wrapped:years');
  const years = $derived(yearsRes.data ?? []);
  const yearsLoading = $derived(yearsRes.loading);
  let year = $state(thisYear);
  const wrapped = new Resource<Wrapped>([]);
  const w = $derived(wrapped.dataKey === `wrapped:${year}` ? (wrapped.data ?? null) : null);
  const loading = $derived(!w && !wrapped.error && years.includes(year));
  const error = $derived(wrapped.error ?? yearsRes.error);
  let card = $state(0);
  $effect(() => { void yearsRes.load('wrapped:years', () => api.statsWrappedYears()); });
  $effect(() => { if (years.length && !years.includes(year)) year = years[0]; });

  async function load() {
    card = 0;
    await wrapped.load(`wrapped:${year}`, () => api.statsWrapped({ year }));
  }
  $effect(() => { year; if (years.includes(year)) void load(); });

  const empty = $derived(!!w && w.totalPlays === 0);
  const TOTAL = 6;
  const maxMonth = $derived(Math.max(1, ...(w?.listenedMsByMonth ?? [1])));
  const bestMonth = $derived(w ? w.listenedMsByMonth.indexOf(Math.max(...w.listenedMsByMonth)) : 0);
  const MONTH_FULL = ['janeiro', 'fevereiro', 'março', 'abril', 'maio', 'junho', 'julho', 'agosto', 'setembro', 'outubro', 'novembro', 'dezembro'];
  const go = (d: number) => (card = Math.max(0, Math.min(TOTAL - 1, card + d)));

  function fmtDay(d: string) { const [y, m, dd] = d.split('-').map(Number); return new Date(y, m - 1, dd).toLocaleDateString('pt-BR', { day: 'numeric', month: 'long' }); }
</script>

<div class="mb-4 flex items-center justify-between gap-3">
  <h1 class="text-2xl font-bold tracking-tight md:text-3xl">Retrospectiva</h1>
  <div class="flex items-center gap-2">
    <label class="sr-only" for="year">Ano</label>
    <select id="year" class="field !w-auto" bind:value={year}>{#each years as y}<option value={y}>{y}</option>{/each}</select>
  </div>
</div>

{#if yearsLoading}
  <SkeletonList variant="card" label="Montando sua retrospectiva" />
{:else if error && !years.length}
  <State kind="error" text={error} />
{:else if !years.length}
  <State kind="empty" icon="sparkle" title="Ainda sem retrospectiva" text="Ouça músicas para criar sua primeira retrospectiva." />
{:else if loading}
  <SkeletonList variant="card" label="Montando sua retrospectiva" />
{:else if error}
  <State kind="error" text={error} onretry={load} />
{:else if !w || empty}
  <State kind="empty" icon="sparkle" title="Sem dados em {year}" text="Ouça músicas durante o ano e a retrospectiva aparece aqui." />
{:else}
  <div class="mx-auto max-w-xl">
    <div class="card relative aspect-[4/5] overflow-hidden sm:aspect-[5/6]" role="group" aria-roledescription="cartão" aria-label="Cartão {card + 1} de {TOTAL}">
      {#key card}
        {#if card === 0}
          <MeshGradient class="h-full">
            <div class="flex h-full min-h-[28rem] flex-col justify-end p-6 md:p-8">
              <p class="eyebrow !text-fg/80">Seu ano em música</p>
              <p class="text-8xl font-black leading-none tracking-tighter text-grad md:text-9xl">{w.year}</p>
              <p class="mt-4 max-w-xs text-lg text-fg/90">Vamos ver o que tocou na sua discoteca.</p>
            </div>
          </MeshGradient>
        {:else if card === 1}
          <div class="flex h-full flex-col justify-center gap-2 p-6 md:p-8" style="background:var(--grad-ember)">
            <p class="eyebrow">Você ouviu</p>
            <p class="tnum text-7xl font-black leading-none tracking-tighter text-grad md:text-8xl">{fmtMinutes(w.totalListenedMs)}</p>
            <p class="text-xl font-semibold">minutos de música</p>
            <p class="mt-4 text-sm text-muted">São {fmtDuration(w.totalListenedMs)}, em {fmtNumber(w.totalPlays)} reproduções de {fmtNumber(w.distinctTracks)} faixas e {fmtNumber(w.distinctArtists)} artistas. Você ainda adicionou {fmtNumber(w.tracksAdded)} faixas novas.</p>
          </div>
        {:else if card === 2}
          <div class="flex h-full flex-col gap-3 overflow-y-auto p-6 md:p-8" style="background:var(--grad-ember)">
            <p class="eyebrow">Suas faixas do ano</p>
            {#if w.topTracks[0]}
              <div class="flex items-center gap-4">
                <Cover path={w.topTracks[0].track.coverPath} seed={w.topTracks[0].track.id} size={96} rounded="0.5rem" />
                <div class="min-w-0"><p class="text-xs text-lava-text">Nº 1</p><p class="text-xl font-bold leading-tight">{w.topTracks[0].track.title}</p><p class="text-sm text-muted">{w.topTracks[0].track.artistName} · {w.topTracks[0].plays} vezes</p></div>
              </div>
            {/if}
            <ol class="mt-2 flex flex-col">
              {#each w.topTracks as r, i (r.track.id)}
                <li>
                  <button class="flex w-full items-center gap-3 rounded-lg border-b border-line px-1 py-2 text-left hover:bg-s2" onclick={() => player.play(r.track.id, w!.topTracks.map((x) => x.track.id))} aria-label="Tocar {r.track.title}">
                    <span class="tnum w-6 text-2xl font-black text-grad">{i + 1}</span>
                    <span class="min-w-0 flex-1"><span class="block truncate text-sm font-medium">{r.track.title}</span><span class="block truncate text-xs text-muted">{r.track.artistName}</span></span>
                    <span class="tnum text-xs text-muted">{r.plays}×</span>
                  </button>
                </li>
              {/each}
            </ol>
          </div>
        {:else if card === 3}
          <div class="flex h-full flex-col gap-3 overflow-y-auto p-6 md:p-8" style="background:var(--grad-ember)">
            <p class="eyebrow">Seus artistas</p>
            {#if w.topArtists[0]}
              <a href={href(paths.artist(w.topArtists[0].artistId))} class="block">
                <p class="text-xs text-lava-text">Nº 1</p>
                <p class="text-4xl font-black leading-tight tracking-tight text-grad md:text-5xl">{w.topArtists[0].name}</p>
                <p class="mt-1 text-sm text-muted">{fmtMinutes(w.topArtists[0].listenedMs)} minutos · {w.topArtists[0].plays} reproduções</p>
              </a>
            {/if}
            <ol class="mt-2">
              {#each w.topArtists.slice(1) as a, i (a.artistId)}
                <li class="flex items-center gap-3 border-b border-line py-2">
                  <span class="tnum w-6 text-2xl font-black text-grad">{i + 2}</span>
                  <a class="min-w-0 flex-1 truncate font-medium hover:underline" href={href(paths.artist(a.artistId))}>{a.name}</a>
                  <span class="tnum text-xs text-muted">{fmtMinutes(a.listenedMs)} min</span>
                </li>
              {/each}
            </ol>
          </div>
        {:else if card === 4}
          <div class="flex h-full flex-col gap-4 p-6 md:p-8" style="background:var(--grad-ember)">
            <div>
              <p class="eyebrow">Mês a mês</p>
              <p class="mt-1 text-2xl font-bold">Seu pico foi em <span class="text-grad">{MONTH_FULL[bestMonth]}</span></p>
              <p class="text-sm text-muted">{fmtMinutes(w.listenedMsByMonth[bestMonth])} minutos ouvidos no mês.</p>
            </div>
            <div class="flex min-h-0 flex-1 items-end gap-1.5" role="img" aria-label="Minutos ouvidos por mês: {w.listenedMsByMonth.map((m, i) => `${MONTHS[i]} ${fmtMinutes(m)}`).join(', ')}">
              {#each w.listenedMsByMonth as ms, i}
                <div class="flex h-full min-w-0 flex-1 flex-col justify-end gap-1.5 text-center">
                  <span class="tnum text-[0.625rem] text-faint">{fmtMinutes(ms)}</span>
                  <div class="rounded-t-md" style="height:{Math.max(2, (ms / maxMonth) * 78)}%;background:{i === bestMonth ? 'var(--grad-lava)' : 'var(--surface-3)'}"></div>
                  <span class="text-[0.6875rem] {i === bestMonth ? 'text-lava-text' : 'text-muted'}">{MONTHS[i]}</span>
                </div>
              {/each}
            </div>
          </div>
        {:else}
          <MeshGradient class="h-full">
            <div class="flex h-full min-h-[28rem] flex-col justify-end gap-3 p-6 md:p-8">
              <p class="eyebrow !text-fg/80">Resumo de {w.year}</p>
              {#if w.busiestDay}
                <p class="text-lg">Seu dia mais intenso foi <strong class="text-grad">{fmtDay(w.busiestDay.date)}</strong>, com {fmtDuration(w.busiestDay.listenedMs)} de música.</p>
              {/if}
              <dl class="grid grid-cols-2 gap-3">
                <div><dt class="text-xs text-fg/70">Minutos</dt><dd class="tnum text-3xl font-black">{fmtMinutes(w.totalListenedMs)}</dd></div>
                <div><dt class="text-xs text-fg/70">Reproduções</dt><dd class="tnum text-3xl font-black">{fmtNumber(w.totalPlays)}</dd></div>
                <div><dt class="text-xs text-fg/70">Faixas</dt><dd class="tnum text-3xl font-black">{fmtNumber(w.distinctTracks)}</dd></div>
                <div><dt class="text-xs text-fg/70">Artistas</dt><dd class="tnum text-3xl font-black">{fmtNumber(w.distinctArtists)}</dd></div>
              </dl>
            </div>
          </MeshGradient>
        {/if}
      {/key}
    </div>

    <div class="mt-4 flex items-center justify-between">
      <button class="btn" onclick={() => go(-1)} disabled={card === 0} aria-label="Cartão anterior"><Icon name="left" size={16} />Anterior</button>
      <div class="flex gap-1.5" role="tablist" aria-label="Cartões">
        {#each Array(TOTAL) as _, i}
          <button role="tab" aria-selected={i === card} aria-label="Ir para o cartão {i + 1}" class="h-2 w-6 origin-left rounded-full border-0 transition-transform {i === card ? 'scale-x-100' : 'scale-x-35'}" style="background:{i === card ? 'var(--lava)' : 'var(--surface-3)'}" onclick={() => (card = i)}></button>
        {/each}
      </div>
      <button class="btn btn-primary" onclick={() => go(1)} disabled={card === TOTAL - 1} aria-label="Próximo cartão">Próximo<Icon name="down" size={16} class="-rotate-90" /></button>
    </div>
  </div>
{/if}
