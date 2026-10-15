// @req RNF-04
// Teclado de uma lista de opções (Dropdown): devolve o próximo índice ativo, 'commit' para
// confirmar a opção ativa, ou null quando a tecla não é da lista. Sem DOM, para rodar nos testes.

export type ListboxMove = number | 'commit' | null;

const fold = (text: string) => text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

export function listboxKey(key: string, active: number, labels: string[]): ListboxMove {
  const last = labels.length - 1;
  if (last < 0) return null;
  switch (key) {
    case 'ArrowDown': return Math.min(last, active + 1);
    case 'ArrowUp': return Math.max(0, active - 1);
    case 'Home': return 0;
    case 'End': return last;
    case 'Enter':
    case ' ': return 'commit';
  }
  if (key.length !== 1) return null;
  // Inicial: pula para a próxima opção que começa com a letra, dando a volta na lista.
  const letter = fold(key);
  for (let step = 1; step <= labels.length; step++) {
    const index = (active + step) % labels.length;
    if (fold(labels[index]).startsWith(letter)) return index;
  }
  return null;
}
