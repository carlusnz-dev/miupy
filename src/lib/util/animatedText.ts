// @req RNF-08
/** Divide o texto em palavras e cada palavra em letras (cluster de grafemas, não unidade UTF-16). */
export function splitWords(text: string): string[][] {
  const letters = (word: string) => (typeof Intl.Segmenter === 'function'
    ? Array.from(new Intl.Segmenter('pt-BR', { granularity: 'grapheme' }).segment(word), (s) => s.segment)
    : Array.from(word));
  return text.split(/\s+/).filter(Boolean).map(letters);
}

/** Atraso (ms) de cada letra, em ordem de leitura; o total é limitado para títulos longos não demorarem. */
export function letterDelays(words: string[][], step = 22, maxTotal = 520): number[][] {
  const count = words.reduce((n, w) => n + w.length, 0);
  const each = count > 1 ? Math.min(step, maxTotal / (count - 1)) : 0;
  let i = 0;
  return words.map((w) => w.map(() => Math.round(i++ * each)));
}
