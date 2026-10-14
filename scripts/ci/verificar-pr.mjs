#!/usr/bin/env node
// Valida título, branch de origem e destino de um PR.
// Uso: node scripts/ci/verificar-pr.mjs --head <branch> --base <branch> --titulo "<título>"
import { parseArgs } from 'node:util';
import { validarPr } from './regras.mjs';

const { values } = parseArgs({
  options: { head: { type: 'string' }, base: { type: 'string' }, titulo: { type: 'string' } },
});
if (!values.head || !values.base || values.titulo === undefined) {
  console.error('uso: verificar-pr.mjs --head <branch> --base <branch> --titulo "<título>"');
  process.exit(2);
}
const erros = validarPr({ head: values.head, base: values.base, titulo: values.titulo });
if (erros.length) {
  for (const e of erros) console.error(`erro: ${e}`);
  console.error('Regras em docs/contribuicao/git.md.');
  process.exit(1);
}
console.log(`ok: ${values.head} -> ${values.base}, título "${values.titulo}".`);
