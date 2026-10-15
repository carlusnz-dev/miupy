// @req RF-39
export const RIGHT_PANEL_KEY = 'miupy.rightPanel';
/** Largura (px) a partir da qual o painel aberto ocupa uma coluna; abaixo disso abre por cima. */
export const RIGHT_PANEL_WIDE = 1280;

/** `null` = o usuário nunca escolheu; vale o padrão pela largura da janela. */
export function parseRightPanel(raw: string | null | undefined): boolean | null {
  return raw === '1' ? true : raw === '0' ? false : null;
}

export function rightPanelOpen(choice: boolean | null, windowWidth: number): boolean {
  return choice ?? windowWidth >= RIGHT_PANEL_WIDE;
}

/** Faixas a mostrar no painel: a atual e as próximas, com o índice absoluto na fila. */
export function upcoming<T>(queue: T[], index: number): { track: T; at: number }[] {
  const from = Math.max(0, index);
  return queue.slice(from).map((track, i) => ({ track, at: from + i }));
}
