// Regras de fluxo do miupy (docs/contribuicao/git.md), sem dependências.
// Funções puras: devolvem uma lista de erros (vazia = ok). Usadas por verificar-pr.mjs e
// verificar-versoes.mjs, e testadas em regras.test.mjs.

export const TIPOS = ['feat', 'fix', 'perf', 'refactor', 'docs', 'test', 'build', 'ci', 'chore', 'style', 'revert'];
export const ESCOPOS = ['core', 'audio', 'media', 'shell', 'ui', 'ipc', 'db', 'docs', 'deps'];
export const PREFIXOS_BRANCH = ['feat', 'fix', 'chore', 'docs', 'refactor', 'test', 'hotfix'];

const SEMVER = /^(\d+)\.(\d+)\.(\d+)$/;
const TITULO = new RegExp(`^(${TIPOS.join('|')})(?:\\(([a-z]+)\\))?(!)?: (.+)$`);
const BRANCH = new RegExp(`^(${PREFIXOS_BRANCH.join('|')})/[a-z0-9]+(?:-[a-z0-9]+)*$`);
const TITULO_RETORNO_HOTFIX = /^chore: traz hotfix v\d+\.\d+\.\d+ para develop$/;
const TITULO_VERSAO = /^chore\(release\): v(\d+\.\d+\.\d+)$/;

export function validarTitulo(titulo) {
  const m = TITULO.exec(titulo);
  if (!m) {
    return [`título fora do formato "<tipo>(<escopo>): <descrição>": "${titulo}". Tipos: ${TIPOS.join(', ')}.`];
  }
  const [, tipo, escopo, , descricao] = m;
  const erros = [];
  // `release` só existe para o PR de versão (chore(release): vX.Y.Z).
  const escoposOk = tipo === 'chore' ? [...ESCOPOS, 'release'] : ESCOPOS;
  if (escopo && !escoposOk.includes(escopo)) {
    erros.push(`escopo "${escopo}" inválido. Escopos: ${escoposOk.join(', ')}.`);
  }
  if (descricao.length > 72) {
    erros.push('descrição com mais de 72 caracteres.');
  }
  if (/^[A-ZÀ-Ý]/.test(descricao)) erros.push('a descrição começa em minúscula.');
  if (/[.]$/.test(descricao)) erros.push('a descrição não termina com ponto final.');
  return erros;
}

export function validarBranch(branch) {
  if (BRANCH.test(branch)) return [];
  return [
    `branch "${branch}" fora do padrão <tipo>/<nome-em-minusculas-com-hifens>. Tipos: ${PREFIXOS_BRANCH.join(', ')}.`,
  ];
}

// Regras de origem e destino. `head` e `base` são nomes de branch.
export function validarFluxo({ head, base, titulo }) {
  if (base === 'main') {
    if (head === 'develop') {
      return TITULO_VERSAO.test(titulo)
        ? []
        : [`PR de versão develop -> main deve se chamar "chore(release): vX.Y.Z" (veio "${titulo}").`];
    }
    if (head.startsWith('hotfix/')) return validarBranch(head);
    return [`PR para main só vem de develop ou de hotfix/* (veio de "${head}").`];
  }
  if (base === 'develop') {
    if (head === 'main') {
      return TITULO_RETORNO_HOTFIX.test(titulo)
        ? []
        : [`main -> develop só para trazer hotfix; o título deve ser "chore: traz hotfix vX.Y.Z para develop" (veio "${titulo}").`];
    }
    if (head === 'develop') return ['develop não pode ser aberto contra si mesmo.'];
    if (head.startsWith('hotfix/')) {
      return ['hotfix/* vai para main; depois main -> develop (ver docs/contribuicao/git.md).'];
    }
    return validarBranch(head);
  }
  return [`base "${base}" não é develop nem main.`];
}

export function validarPr({ head, base, titulo }) {
  return [...validarTitulo(titulo), ...validarFluxo({ head, base, titulo })];
}

export function parseSemver(v) {
  const m = SEMVER.exec(v);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

export function compararSemver(a, b) {
  const x = parseSemver(a);
  const y = parseSemver(b);
  for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] - y[i];
  return 0;
}

// `versoes`: { 'Cargo.toml': '0.2.1', ... }. `ultimaTag`: '0.2.1' (sem o v) ou null.
// modo 'maior': versão tem de ser maior que a última tag (PR de versão).
// modo 'igual': versão tem de ser igual à tag `esperada` (workflow de release).
export function validarVersoes({ versoes, ultimaTag = null, modo = 'maior', esperada = null }) {
  const erros = [];
  const entradas = Object.entries(versoes);
  for (const [arquivo, v] of entradas) {
    if (!parseSemver(v ?? '')) erros.push(`${arquivo}: versão "${v}" não é X.Y.Z.`);
  }
  if (erros.length) return erros;
  const distintas = new Set(entradas.map(([, v]) => v));
  if (distintas.size > 1) {
    erros.push(`versões diferentes: ${entradas.map(([a, v]) => `${a}=${v}`).join(', ')}.`);
    return erros;
  }
  const versao = entradas[0][1];
  if (modo === 'igual') {
    if (versao !== esperada) erros.push(`a tag é v${esperada}, mas os arquivos dizem ${versao}.`);
  } else if (ultimaTag && compararSemver(versao, ultimaTag) <= 0) {
    erros.push(`versão ${versao} não é maior que a última tag v${ultimaTag}.`);
  }
  return erros;
}

// Maior tag vX.Y.Z de uma lista de nomes de tag.
export function ultimaTag(tags) {
  const validas = tags.map((t) => /^v(\d+\.\d+\.\d+)$/.exec(t)?.[1]).filter(Boolean);
  if (!validas.length) return null;
  return validas.sort(compararSemver).at(-1);
}
