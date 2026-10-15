#!/usr/bin/env node
// Confere que Cargo.toml (workspace), package.json e src-tauri/tauri.conf.json têm a mesma versão.
// Uso:
//   verificar-versoes.mjs --maior-que-a-ultima-tag      (PR de versão: versão > maior tag v*)
//   verificar-versoes.mjs --tag vX.Y.Z                  (release: versão == tag)
//   verificar-versoes.mjs                               (só confere que são iguais)
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { validarVersoes, ultimaTag } from './regras.mjs';

const { values } = parseArgs({
  options: { 'maior-que-a-ultima-tag': { type: 'boolean' }, tag: { type: 'string' }, raiz: { type: 'string' } },
});
const raiz = values.raiz ?? join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const ler = (f) => readFileSync(join(raiz, f), 'utf8');

const cargo = /\[workspace\.package\][^[]*?\nversion\s*=\s*"([^"]+)"/.exec(ler('Cargo.toml'))?.[1];
const versoes = {
  'Cargo.toml': cargo,
  'package.json': JSON.parse(ler('package.json')).version,
  'src-tauri/tauri.conf.json': JSON.parse(ler('src-tauri/tauri.conf.json')).version,
};

let entrada;
if (values.tag) {
  const m = /^v(\d+\.\d+\.\d+)$/.exec(values.tag);
  if (!m) {
    console.error(`erro: tag "${values.tag}" não é vX.Y.Z.`);
    process.exit(1);
  }
  entrada = { versoes, modo: 'igual', esperada: m[1] };
} else if (values['maior-que-a-ultima-tag']) {
  const tags = execFileSync('git', ['-C', raiz, 'tag', '--list', 'v*'], { encoding: 'utf8' }).split('\n');
  entrada = { versoes, modo: 'maior', ultimaTag: ultimaTag(tags) };
} else {
  entrada = { versoes, modo: 'maior' };
}
const erros = validarVersoes(entrada);
if (erros.length) {
  for (const e of erros) console.error(`erro: ${e}`);
  process.exit(1);
}
console.log(`ok: versão ${versoes['package.json']} nos três arquivos.`);
