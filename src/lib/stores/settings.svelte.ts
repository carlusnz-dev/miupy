// @req RF-17
import { api, type LavaTheme, type Settings } from '../ipc';

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
  loaded = $state(false);

  /** Carrega as configurações salvas. O cache de localStorage já evitou o flash inicial. */
  async init() {
    const dom = document.documentElement.dataset.lava as LavaTheme | undefined;
    if (dom) this.lava = dom;
    try {
      const s = await api.settingsGet();
      this.lava = s.lavaTheme;
      applyLava(s.lavaTheme);
    } catch { /* mantém o cache */ }
    this.loaded = true;
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
