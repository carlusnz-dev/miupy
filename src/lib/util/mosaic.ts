// @req RF-22
export function mosaicCells(paths: string[]): string[] {
  const p = paths.slice(0, 4);
  if (p.length === 1) return p;
  if (p.length === 2) return [p[0], p[1], p[1], p[0]];
  if (p.length === 3) return [p[0], p[1], p[2], p[0]];
  return p;
}
