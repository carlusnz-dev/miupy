// @req RF-29
// Fecha um popup com Escape ou com clique fora dele. A lógica não depende do DOM: recebe o alvo
// dos eventos (o `document`) e um teste de "está dentro", para rodar também nos testes.

export type DismissReason = 'escape' | 'outside';
type Events = Pick<EventTarget, 'addEventListener' | 'removeEventListener'>;

export function dismissOnEscapeOrOutside(
  root: Events,
  isInside: (target: EventTarget | null) => boolean,
  close: (reason: DismissReason) => void,
): () => void {
  const onKey = (event: Event) => {
    if ((event as KeyboardEvent).key === 'Escape') close('escape');
  };
  const onPointer = (event: Event) => {
    if (!isInside(event.target)) close('outside');
  };
  root.addEventListener('keydown', onKey);
  root.addEventListener('pointerdown', onPointer, true);
  return () => {
    root.removeEventListener('keydown', onKey);
    root.removeEventListener('pointerdown', onPointer, true);
  };
}

/** Action do Svelte: o elemento é a área "dentro" (botão e popup); fora dela, ou Escape, chama `close`. */
export function dismissable(node: HTMLElement, close: (reason: DismissReason) => void) {
  let current = close;
  const off = dismissOnEscapeOrOutside(document, (target) => target instanceof Node && node.contains(target), (r) => current(r));
  return {
    update(next: (reason: DismissReason) => void) { current = next; },
    destroy: off,
  };
}
