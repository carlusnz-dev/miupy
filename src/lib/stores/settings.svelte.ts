// @req RF-17
import { api, type LavaTheme, type Normalization, type Settings } from '../ipc';

const KEY = 'miupy.lava';

export function applyLava(t: LavaTheme) {
  document.documentElement.dataset.lava = t;
  try { localStorage.setItem(KEY, t); } catch { /* sem storage: ok */ }
  // mantém a cor da barra de status do WebView coerente com o tema
  const meta = document.querySelector('meta[name="theme-color"]');
  const bg = getComputedStyle(document.documentElement).getPropertyValue('--bg').trim();
  if (meta && bg) meta.setAttribute('content', bg);
}

class SettingsStore {
  lava = $state<LavaTheme>('orange');
  normalization = $state<Normalization>('normal');
  eqPresetId = $state('flat');
  loaded = $state(false);

  /** Carrega as configurações salvas. O cache de localStorage já evitou o flash inicial. */
  async init() {
    const dom = document.documentElement.dataset.lava as LavaTheme | undefined;
    if (dom) this.lava = dom;
    try {
      const s = await api.settingsGet();
      this.lava = s.lavaTheme;
      this.normalization = s.normalization;
      this.eqPresetId = s.eqPresetId;
      applyLava(s.lavaTheme);
    } catch { /* mantém o cache */ }
    this.loaded = true;
  }

  // @req RF-28
  async setNormalization(value: Normalization) {
    const saved = await api.settingsUpdate({ settings: { normalization: value } });
    this.normalization = saved.normalization;
  }

  // @req RF-29
  async setEqualizer(id: string) {
    const saved=await api.settingsUpdate({settings:{eqPresetId:id}});
    this.eqPresetId=saved.eqPresetId;
  }

  /** Aplica na hora (pré-visualização imediata) e persiste em seguida. */
  async setLava(t: LavaTheme) {
    this.lava = t;
    applyLava(t);
    const s: Partial<Settings> = { lavaTheme: t };
    await api.settingsUpdate({ settings: s });
  }
}
export const settings = new SettingsStore();
