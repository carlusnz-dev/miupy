// @req RF-05
const DAY = 86_400_000;

/** "hoje", "há 3 dias", "há 2 meses", "há 1 ano": o tempo desde `addedAt` na maior unidade que cabe. */
export function fmtAddedAgo(addedAt: number, now = Date.now()): string {
  const days = Math.max(0, Math.floor((now - addedAt) / DAY));
  if (days < 1) return 'hoje';
  if (days < 30) return `há ${days} ${days === 1 ? 'dia' : 'dias'}`;
  if (days < 365) { const m = Math.floor(days / 30); return `há ${m} ${m === 1 ? 'mês' : 'meses'}`; }
  const y = Math.floor(days / 365);
  return `há ${y} ${y === 1 ? 'ano' : 'anos'}`;
}
