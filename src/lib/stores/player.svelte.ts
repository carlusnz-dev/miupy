// @req RF-01 RF-02 RF-03 RF-04 RNF-13
import { api, type PlayerState, type Track } from '../ipc';
import { nextShuffle } from '../util/shuffle';
import { notifyError } from './toast.svelte';
import { scores } from './scores.svelte';
import { deepEqual } from '../util/diff';
import { affectsCurrentTrack, affectsScores } from '../util/libraryEvents';

const EMPTY: PlayerState = { status: 'stopped', track: null, positionMs: 0, durationMs: 0, volume: 0.8, muted: false, queue: [], queueIndex: -1, streaming: false, shuffle: 'off' };

class PlayerStore {
  state = $state<PlayerState>({ ...EMPTY });
  /** Faixas da fila resolvidas (a fila do contrato só traz ids). */
  queueTracks = $state<Track[]>([]);
  private started = false;
  private lastQueueKey = '';

  get track() { return this.state.track; }
  get isPlaying() { return this.state.status === 'playing'; }
  get isLoading() { return this.state.status === 'loading'; }
  get active() { return this.state.track !== null; }
  get progress() { return this.state.durationMs > 0 ? this.state.positionMs / this.state.durationMs : 0; }
  get hasNext() { return this.state.queueIndex >= 0 && this.state.queueIndex + 1 < this.state.queue.length; }

  async start() {
    if (this.started) return;
    this.started = true;
    await api.on('player://state', (s) => this.apply(s));
    await api.on('player://progress', (p) => {
      this.state.positionMs = p.positionMs;
      this.state.durationMs = p.durationMs;
    });
    await api.on('player://error', (e) => notifyError({ message: e.message })); // @req RF-18
    // favorito alterado fora do player (biblioteca) reflete na faixa atual; foto de artista, playlist
    // e afins não mexem nela, então não custam consulta nem repintura (RNF-13)
    await api.on('library://changed', (e) => {
      if (affectsScores(e?.reason)) scores.invalidate();
      if (affectsCurrentTrack(e?.reason)) void this.refreshCurrent();
    });
    try { this.apply(await api.playerGetState()); } catch (e) { notifyError(e); }
  }

  private apply(s: PlayerState) {
    const before = this.state.track?.id;
    if (s.track?.id !== before) scores.invalidate([before, s.track?.id].filter((id): id is string => !!id)); // a anterior ganhou um evento
    this.merge(s);
    const key = s.queue.join(',');
    if (key !== this.lastQueueKey) { this.lastQueueKey = key; void this.resolveQueue(s.queue); }
  }

  /**
   * Aplica o estado campo a campo: o que não mudou não notifica ninguém. Trocar `this.state` inteiro
   * reavaliava todos os derivados que leem o player a cada evento (RNF-13).
   */
  private merge(s: PlayerState) {
    const cur = this.state;
    cur.status = s.status;
    cur.positionMs = s.positionMs;
    cur.durationMs = s.durationMs;
    cur.volume = s.volume;
    cur.muted = s.muted;
    cur.queueIndex = s.queueIndex;
    cur.streaming = s.streaming;
    cur.shuffle = s.shuffle;
    if (!deepEqual(cur.track, s.track)) cur.track = s.track;
    if (!deepEqual(cur.queue, s.queue)) cur.queue = s.queue;
  }

  private async resolveQueue(ids: string[]) {
    try { const out = await api.playerGetQueue(); if (ids.join(',') === this.lastQueueKey) this.queueTracks = out; }
    catch (e) { notifyError(e); }
  }

  private async refreshCurrent() {
    const t = this.state.track;
    if (!t) return;
    try {
      const fresh = await api.libraryGetTrack({ trackId: t.id });
      if (this.state.track?.id === fresh.id && !deepEqual(this.state.track, fresh)) this.state.track = fresh;
      const at = this.queueTracks.findIndex((q) => q.id === fresh.id);
      if (at >= 0 && !deepEqual(this.queueTracks[at], fresh)) this.queueTracks = this.queueTracks.map((q) => (q.id === fresh.id ? fresh : q));
    } catch { /* ignora */ }
  }

  private async run(p: Promise<PlayerState>) {
    try { this.apply(await p); } catch (e) {
      if (e && typeof e === 'object' && 'code' in e && e.code === 'network') notifyError({ message: 'Sem conexão: baixe a faixa para ouvir offline.' });
      else notifyError(e);
    }
  }

  cycleShuffle() { return this.run(api.playerSetShuffle({ mode: nextShuffle(this.state.shuffle) })); } // @req RF-24
  play(trackId: string, queue?: string[]) { return this.run(api.playerPlay({ trackId, queue })); }
  pause() { return this.run(api.playerPause()); }
  resume() { return this.run(api.playerResume()); }
  stop() { return this.run(api.playerStop()); }
  toggle() {
    if (this.state.status === 'playing' || this.state.status === 'loading') return this.pause();
    if (this.state.status === 'paused') return this.resume();
    return Promise.resolve();
  }
  next() { return this.run(api.playerNext()); }
  previous() { return this.run(api.playerPrevious()); }
  seek(positionMs: number) {
    this.state.positionMs = positionMs; // otimista; o evento confirma
    return this.run(api.playerSeek({ positionMs }));
  }
  seekBy(deltaMs: number) {
    return this.seek(Math.max(0, Math.min(this.state.durationMs, this.state.positionMs + deltaMs)));
  }
  setVolume(volume: number) {
    const v = Math.max(0, Math.min(1, volume));
    this.state.volume = v;
    return this.run(api.playerSetVolume({ volume: v }));
  }
  setMuted(muted: boolean) { return this.run(api.playerSetMuted({ muted })); }
  toggleMuted() { return this.setMuted(!this.state.muted); }
}
export const player = new PlayerStore();
