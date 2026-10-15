// Diálogos globais: seletor de playlist e confirmação.
// @req RF-06
class Ui {
  picker = $state<{ trackIds: string[] } | null>(null);
  confirmState = $state<{ title: string; text: string; confirmLabel: string; danger: boolean; option: string | null; checked: boolean; resolve: (v: boolean) => void } | null>(null);

  pickPlaylist(trackIds: string[]) { if (trackIds.length) this.picker = { trackIds }; }

  confirm(title: string, text: string, confirmLabel = 'Confirmar', danger = false, option: string | null = null): Promise<boolean> {
    return new Promise((resolve) => { this.confirmState = { title, text, confirmLabel, danger, option, checked: false, resolve }; });
  }
  /** Como `confirm`, mas com uma caixa de marcar (`option`); devolve `null` se cancelou, senão o valor da caixa. */
  async confirmOption(title: string, text: string, confirmLabel: string, option: string): Promise<boolean | null> {
    const ok = await this.confirm(title, text, confirmLabel, true, option);
    return ok ? this.lastChecked : null;
  }
  private lastChecked = false;
  answer(v: boolean) { this.lastChecked = this.confirmState?.checked ?? false; this.confirmState?.resolve(v); this.confirmState = null; }
}
export const ui = new Ui();
