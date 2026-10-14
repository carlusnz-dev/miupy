// Diálogos globais: seletor de playlist e confirmação.
// @req RF-06
class Ui {
  picker = $state<{ trackIds: string[] } | null>(null);
  confirmState = $state<{ title: string; text: string; confirmLabel: string; danger: boolean; resolve: (v: boolean) => void } | null>(null);

  pickPlaylist(trackIds: string[]) { if (trackIds.length) this.picker = { trackIds }; }

  confirm(title: string, text: string, confirmLabel = 'Confirmar', danger = false): Promise<boolean> {
    return new Promise((resolve) => { this.confirmState = { title, text, confirmLabel, danger, resolve }; });
  }
  answer(v: boolean) { this.confirmState?.resolve(v); this.confirmState = null; }
}
export const ui = new Ui();
