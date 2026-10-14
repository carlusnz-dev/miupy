// Seleção múltipla de faixas (biblioteca e playlists). @req RF-10
export class Selection {
  ids = $state<string[]>([]);
  get count() { return this.ids.length; }
  has(id: string) { return this.ids.includes(id); }
  toggle(id: string) { this.ids = this.has(id) ? this.ids.filter((x) => x !== id) : [...this.ids, id]; }
  set(ids: string[]) { this.ids = [...ids]; }
  clear() { this.ids = []; }
}
