export interface Toast { id: number; kind: 'info' | 'error' | 'success'; text: string; }

class Toasts {
  items = $state<Toast[]>([]);
  private n = 0;
  push(text: string, kind: Toast['kind'] = 'info', ms = 4000) {
    const id = ++this.n;
    this.items.push({ id, kind, text });
    setTimeout(() => this.dismiss(id), ms);
  }
  dismiss(id: number) { this.items = this.items.filter((t) => t.id !== id); }
}
export const toasts = new Toasts();
export const notify = (t: string) => toasts.push(t, 'success');
export const notifyError = (e: unknown) => {
  const m = e && typeof e === 'object' && 'message' in e ? String((e as { message: unknown }).message) : 'Algo deu errado.';
  toasts.push(m, 'error', 6000);
};
