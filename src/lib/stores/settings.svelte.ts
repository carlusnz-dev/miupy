// @req RF-17
import { api, type LavaTheme, type Normalization, type Settings } from '../ipc';
import { parseRightPanel, RIGHT_PANEL_KEY } from '../util/rightPanel';
import { clampSidebarWidth, parseCollapsed, SIDEBAR_COLLAPSED_KEY, SIDEBAR_KEY } from '../util/sidebar';

const KEY = 'miupy.lava';

export function applyLava(t: LavaTheme) {
  document.documentElement.dataset.lava = t;
  try { localStorage.setItem(KEY, t); } catch { /* sem storage: ok */ }
  // mantém a cor da barra de status do WebView coerente com o tema
  const meta = document.querySelector('meta[name="theme-color"]');
  const bg = getComputedStyle(document.documentElement).getPropertyValue('--bg').trim();
  if (meta && bg) meta.setAttribute('content', bg);
}

function readCollapsed(): boolean {
  try { return parseCollapsed(localStorage.getItem(SIDEBAR_COLLAPSED_KEY)); } catch { return false; }
}

function readRightPanel(): boolean | null {
  try { return parseRightPanel(localStorage.getItem(RIGHT_PANEL_KEY)); } catch { return null; }
}

class SettingsStore {
  lava = $state<LavaTheme>('orange');
  normalization = $state<Normalization>('normal');
  eqPresetId = $state('flat');
  sidebarWidth = $state(280);
  sidebarCollapsed = $state(readCollapsed());
  /** @req RF-39: `null` até o usuário abrir ou recolher o painel da direita. */
  rightPanel = $state<boolean | null>(readRightPanel());
  loaded = $state(false);
  private sidebarSave: Promise<void> = Promise.resolve();
  private sidebarSaveGeneration = 0;

  /** Carrega as configurações salvas. O cache de localStorage já evitou o flash inicial. */
  async init() {
    const dom = document.documentElement.dataset.lava as LavaTheme | undefined;
    if (dom) this.lava = dom;
    try {
      const s = await api.settingsGet();
      this.lava = s.lavaTheme;
      this.normalization = s.normalization;
      this.eqPresetId = s.eqPresetId;
      this.applySidebarWidth(s.sidebarWidth);
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

  // @req RF-33
  applySidebarWidth(width: number) {
    const value = clampSidebarWidth(width);
    this.sidebarWidth = value;
    document.documentElement.style.setProperty('--sidebar-w', `${value}px`);
    try { localStorage.setItem(SIDEBAR_KEY, String(value)); } catch { /* sem storage: ok */ }
  }

  // Chamado somente ao soltar a alça ou depois de uma tecla.
  async saveSidebarWidth(width: number) {
    const value = clampSidebarWidth(width);
    const generation = ++this.sidebarSaveGeneration;
    const save = this.sidebarSave.then(async () => {
      const saved = await api.settingsUpdate({ settings: { sidebarWidth: value } });
      if (generation === this.sidebarSaveGeneration) this.applySidebarWidth(saved.sidebarWidth);
    });
    this.sidebarSave = save.catch(() => {});
    return save;
  }

  // @req RF-38
  setSidebarCollapsed(value: boolean) {
    this.sidebarCollapsed = value;
    try { localStorage.setItem(SIDEBAR_COLLAPSED_KEY, value ? '1' : '0'); } catch { /* sem storage: ok */ }
  }

  // @req RF-39
  setRightPanel(value: boolean) {
    this.rightPanel = value;
    try { localStorage.setItem(RIGHT_PANEL_KEY, value ? '1' : '0'); } catch { /* sem storage: ok */ }
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
