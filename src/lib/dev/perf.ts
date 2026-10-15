// @req RNF-13
// Medidor de fluidez, só de desenvolvimento (importado por main.ts sob import.meta.env.DEV; fica fora
// do bundle de produção). Uso: `await __miupyPerf.run()` no console. Roteiro em docs/contribuicao/desempenho.md.
import { api } from '../ipc';
import { burstSchedule, meetsTarget, summarizeFrames, type FrameSummary } from './perfStats';

export interface StepResult extends FrameSummary { step: string; ok: boolean }
export interface PerfReport extends FrameSummary {
  engine: string;
  mock: boolean;
  tracks: number | null;
  /** `null` quando o motor não suporta a API de long tasks (Firefox, WebKit). */
  longTasks: { count: number; totalMs: number; maxMs: number } | null;
  ok: boolean;
  steps: StepResult[];
}

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
const nextFrame = () => new Promise<number>((r) => requestAnimationFrame(r));

function startLongTasks() {
  const supported = typeof PerformanceObserver !== 'undefined' && PerformanceObserver.supportedEntryTypes?.includes('longtask');
  if (!supported) return { stop: () => null as PerfReport['longTasks'] };
  const durations: number[] = [];
  const obs = new PerformanceObserver((list) => list.getEntries().forEach((e) => durations.push(e.duration)));
  obs.observe({ entryTypes: ['longtask'] });
  return {
    stop: () => {
      obs.takeRecords().forEach((e) => durations.push(e.duration));
      obs.disconnect();
      return { count: durations.length, totalMs: Math.round(durations.reduce((s, d) => s + d, 0)), maxMs: Math.round(Math.max(0, ...durations)) };
    },
  };
}

/** Tempo de quadro por rAF. Cada `mark(step)` fecha o passo anterior. */
function startFrames() {
  const all: number[] = [];
  const perStep = new Map<string, number[]>();
  let step = 'inicio';
  let running = true;
  let last = performance.now();
  const tick = (now: number) => {
    if (!running) return;
    const d = now - last;
    last = now;
    all.push(d);
    (perStep.get(step) ?? perStep.set(step, []).get(step)!).push(d);
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  return {
    mark: (s: string) => { step = s; },
    stop: () => { running = false; return { all, perStep }; },
  };
}

async function go(hash: string, settleMs = 900) {
  location.hash = hash;
  await sleep(settleMs);
}

async function emitLibraryChanged(reason: string) {
  const mock = (window as unknown as { __miupyMockEmit?: (e: string, p: unknown) => void }).__miupyMockEmit;
  if (mock) { mock('library://changed', { reason }); return; }
  const { emit } = await import('@tauri-apps/api/event');
  await emit('library://changed', { reason });
}

async function scrollMain(ms: number) {
  const main = document.getElementById('conteudo');
  if (!main) return;
  const end = performance.now() + ms;
  let dir = 1;
  while (performance.now() < end) {
    await nextFrame();
    main.scrollTop += dir * 48;
    if (main.scrollTop + main.clientHeight >= main.scrollHeight - 2) dir = -1;
    if (main.scrollTop <= 0) dir = 1;
  }
  main.scrollTop = 0;
}

async function run(): Promise<PerfReport> {
  const lt = startLongTasks();
  const frames = startFrames();
  const list = await api.libraryListTracks();
  const first = list[0];

  frames.mark('0 tocar uma faixa');
  if (first) await api.playerPlay({ trackId: first.id, queue: list.slice(0, 50).map((t) => t.id) });
  await sleep(600);

  const nav: [string, string][] = [
    ['1 Início', '#/'],
    ['2 Discoteca', '#/biblioteca'],
    ['3 Artista', `#/artista/${first?.artistId ?? 'art-1'}`],
    ['4 Playlists', '#/playlists'],
    ['5 Tocando', '#/tocando'],
  ];
  for (const [name, hash] of nav) { frames.mark(name); await go(hash); }

  frames.mark('6 abrir a fila');
  document.querySelector<HTMLElement>('[role="tab"]')?.click();
  await sleep(600);

  frames.mark('7 voltar ao Início');
  await go('#/');

  frames.mark('8 rolar a Discoteca');
  await go('#/biblioteca', 700);
  frames.mark('8 rolar a Discoteca');
  await scrollMain(2000);

  frames.mark('9 rajada de library://changed');
  const reasons = ['metadata_refreshed', 'artist_photo_updated', 'metadata_refreshed', 'artist_updated', 'download_done', 'metadata_refreshed', 'artist_photo_updated', 'metadata_refreshed', 'artist_updated', 'metadata_refreshed'];
  const start = performance.now();
  for (const [i, at] of burstSchedule(10, 2000).entries()) {
    await sleep(Math.max(0, start + at - performance.now()));
    void emitLibraryChanged(reasons[i]);
  }
  await sleep(1000);

  const { all, perStep } = frames.stop();
  // descarta o primeiro quadro (intervalo desde antes do início) de cada série
  const total = summarizeFrames(all.slice(1));
  const steps = [...perStep].map(([step, d]) => { const s = summarizeFrames(d); return { step, ok: meetsTarget(s), ...s }; });
  const report: PerfReport = {
    ...total,
    engine: navigator.userAgent,
    mock: !('__TAURI_INTERNALS__' in window),
    tracks: list.length,
    longTasks: lt.stop(),
    ok: meetsTarget(total),
    steps,
  };
  await go('#/', 100);
  return report;
}

declare global { interface Window { __miupyPerf: { run: typeof run } } }
window.__miupyPerf = { run };
