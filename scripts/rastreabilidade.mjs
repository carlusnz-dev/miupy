#!/usr/bin/env node
// Gera docs/rastreabilidade/matriz.md e matriz.json a partir de:
//   - docs/requisitos/requisitos.md  (linhas de tabela que começam com RF-NN / RNF-NN)
//   - anotações `@req <IDs>` e `@test <IDs>` no código-fonte
// Uso: node scripts/rastreabilidade.mjs [--check]
//   --check: não escreve nada; sai com código 1 se houver ID anotado que não existe nos requisitos.

import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join, relative, extname, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const REQS_FILE = join(ROOT, 'docs/requisitos/requisitos.md');
const OUT_MD = join(ROOT, 'docs/rastreabilidade/matriz.md');
const OUT_JSON = join(ROOT, 'docs/rastreabilidade/matriz.json');
const SCAN_DIRS = ['crates', 'src-tauri', 'src', 'scripts'];
const SCAN_EXT = new Set(['.rs', '.ts', '.js', '.mjs', '.svelte', '.css', '.kt', '.sql', '.html']);
const SKIP_DIRS = new Set(['node_modules', 'target', 'dist', 'gen', '.git', '.svelte-kit']);
const ID_RE = /\bRN?F-\d{2,}\b/g;
const TAG_RE = /@(req|test)\b([^\n]*)/g;

function parseRequirements() {
  const reqs = [];
  for (const line of readFileSync(REQS_FILE, 'utf8').split('\n')) {
    const cells = line.split('|').map((c) => c.trim());
    if (cells.length < 5 || !/^RN?F-\d{2,}$/.test(cells[1])) continue;
    reqs.push({ id: cells[1], title: cells[2], priority: cells[3], description: cells[4] });
  }
  return reqs;
}

function* walk(dir) {
  if (!existsSync(dir)) return;
  for (const name of readdirSync(dir)) {
    if (SKIP_DIRS.has(name)) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) yield* walk(path);
    else if (SCAN_EXT.has(extname(name))) yield path;
  }
}

function scanAnnotations() {
  const found = { req: new Map(), test: new Map() };
  const self = fileURLToPath(import.meta.url);
  for (const dir of SCAN_DIRS) {
    for (const file of walk(join(ROOT, dir))) {
      if (file === self) continue;
      const lines = readFileSync(file, 'utf8').split('\n');
      lines.forEach((text, i) => {
        for (const m of text.matchAll(TAG_RE)) {
          for (const id of m[2].match(ID_RE) ?? []) {
            const bucket = found[m[1]];
            if (!bucket.has(id)) bucket.set(id, []);
            bucket.get(id).push(`${relative(ROOT, file)}:${i + 1}`);
          }
        }
      });
    }
  }
  return found;
}

function status(impl, tests) {
  if (impl.length && tests.length) return 'verificado';
  if (impl.length) return 'implementado';
  if (tests.length) return 'só teste';
  return 'pendente';
}

const refs = (list) => (list.length ? list.map((r) => `\`${r}\``).join('<br>') : '—');

const reqs = parseRequirements();
const found = scanAnnotations();
const known = new Set(reqs.map((r) => r.id));
const unknown = [...new Set([...found.req.keys(), ...found.test.keys()])].filter((id) => !known.has(id)).sort();

const rows = reqs.map((r) => {
  const impl = found.req.get(r.id) ?? [];
  const tests = found.test.get(r.id) ?? [];
  return { ...r, implementation: impl, tests, status: status(impl, tests) };
});

if (process.argv.includes('--check')) {
  if (unknown.length) {
    console.error(`IDs anotados que não existem nos requisitos: ${unknown.join(', ')}`);
    process.exit(1);
  }
  console.log(`ok: ${rows.length} requisitos, nenhum ID desconhecido.`);
  process.exit(0);
}

const count = (s) => rows.filter((r) => r.status === s).length;
const summary = {
  total: rows.length,
  verificado: count('verificado'),
  implementado: count('implementado'),
  soTeste: count('só teste'),
  pendente: count('pendente'),
};

const md = [
  '# Matriz de rastreabilidade',
  '',
  '> Arquivo gerado por `scripts/rastreabilidade.mjs` (`npm run docs:matriz`). Não edite à mão.',
  '',
  `Total: **${summary.total}** · verificado: **${summary.verificado}** · implementado: **${summary.implementado}** · só teste: **${summary.soTeste}** · pendente: **${summary.pendente}**`,
  '',
  '| ID | Requisito | Prior. | Status | Implementação (`@req`) | Testes (`@test`) |',
  '|---|---|---|---|---|---|',
  ...rows.map((r) => `| ${r.id} | ${r.title} | ${r.priority} | ${r.status} | ${refs(r.implementation)} | ${refs(r.tests)} |`),
  '',
  ...(unknown.length ? ['## IDs anotados que não existem nos requisitos', '', ...unknown.map((id) => `- \`${id}\``), ''] : []),
].join('\n');

writeFileSync(OUT_MD, md);
writeFileSync(OUT_JSON, JSON.stringify({ summary, unknownIds: unknown, requirements: rows }, null, 2) + '\n');
console.log(`matriz gerada: ${summary.total} requisitos, ${summary.verificado} verificados, ${summary.implementado} implementados, ${summary.pendente} pendentes.`);
if (unknown.length) console.warn(`aviso: IDs desconhecidos: ${unknown.join(', ')}`);
