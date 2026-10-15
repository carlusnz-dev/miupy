// @req RF-40
// Cores dominantes da capa para o fundo da tela Tocando. `dominantColors` é pura (testável);
// `coverPalette` desenha a imagem num canvas pequeno e devolve `null` se o canvas ficar "tainted".

const SIZE = 32;

/** Quantiza os pixels RGBA em 16 níveis por canal, descarta quase preto, quase branco e cinza, e devolve até `count` cores distintas (hex), as mais frequentes e saturadas primeiro. */
export function dominantColors(rgba: ArrayLike<number>, count = 3): string[] {
  const buckets = new Map<number, { n: number; r: number; g: number; b: number }>();
  for (let i = 0; i + 3 < rgba.length; i += 4) {
    const [r, g, b, a] = [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    if (a < 128 || max < 40 || min > 225 || max - min < 24) continue;
    const key = ((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4);
    const bucket = buckets.get(key) ?? { n: 0, r: 0, g: 0, b: 0 };
    bucket.n++; bucket.r += r; bucket.g += g; bucket.b += b;
    buckets.set(key, bucket);
  }
  const ranked = [...buckets.values()]
    .map((c) => ({ rgb: [c.r / c.n, c.g / c.n, c.b / c.n], score: c.n * (1 + (Math.max(c.r, c.g, c.b) - Math.min(c.r, c.g, c.b)) / c.n / 255) }))
    .sort((x, y) => y.score - x.score);
  const picked: number[][] = [];
  for (const { rgb } of ranked) {
    if (picked.every((p) => Math.hypot(p[0] - rgb[0], p[1] - rgb[1], p[2] - rgb[2]) > 60)) picked.push(rgb);
    if (picked.length === count) break;
  }
  return picked.map(([r, g, b]) => `#${[r, g, b].map((v) => Math.round(v).toString(16).padStart(2, '0')).join('')}`);
}

/** Carrega a capa e extrai as cores; `null` se a imagem não carregar ou o canvas não puder ser lido. */
export async function coverPalette(src: string, crossOrigin?: 'anonymous'): Promise<string[] | null> {
  try {
    const img = new Image();
    if (crossOrigin) img.crossOrigin = crossOrigin;
    img.src = src;
    await img.decode();
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = SIZE;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return null;
    ctx.drawImage(img, 0, 0, SIZE, SIZE);
    return dominantColors(ctx.getImageData(0, 0, SIZE, SIZE).data);
  } catch {
    return null;
  }
}
