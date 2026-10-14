// Testes das regras de fluxo: node --test scripts/ci/
import test from 'node:test';
import assert from 'node:assert/strict';
import { validarTitulo, validarBranch, validarPr, validarVersoes, ultimaTag } from './regras.mjs';

const ok = (erros) => assert.deepEqual(erros, []);
const ruim = (erros) => assert.ok(erros.length > 0, 'esperava erro');

test('títulos bons', () => {
  for (const t of [
    'feat(audio): toca M4A por streaming',
    'fix(media): usa a URL do vídeo para obter o mix',
    'feat(ipc)!: devolve a fila como Track[]',
    'docs: registra a v0.2.1 no relatório',
    'chore(release): v0.3.0',
    'chore: traz hotfix v0.2.2 para develop',
    'ci: adiciona integração contínua',
  ]) ok(validarTitulo(t));
});

test('títulos ruins', () => {
  for (const t of [
    'Adiciona coisa',
    'feature(audio): algo',
    'feat(audio) algo',
    'feat(xpto): algo',
    'feat(audio): Algo com maiúscula',
    'feat(audio): termina com ponto.',
    'feat(audio): ',
    'feat(release): só chore pode usar release',
    `feat: ${'x'.repeat(80)}`,
  ]) ruim(validarTitulo(t));
});

test('branches', () => {
  for (const b of ['feat/recomendacoes', 'fix/42-duracao-m4a', 'hotfix/crash', 'chore/integracao-continua']) ok(validarBranch(b));
  for (const b of ['patch-1', 'feat/Maiuscula', 'feature/x', 'feat/', 'feat/a_b', 'develop', 'main']) ruim(validarBranch(b));
});

test('fluxo de PR', () => {
  ok(validarPr({ head: 'feat/x', base: 'develop', titulo: 'feat(ui): algo' }));
  ok(validarPr({ head: 'develop', base: 'main', titulo: 'chore(release): v0.3.0' }));
  ok(validarPr({ head: 'hotfix/crash', base: 'main', titulo: 'fix(audio): evita crash' }));
  ok(validarPr({ head: 'main', base: 'develop', titulo: 'chore: traz hotfix v0.2.2 para develop' }));
  ruim(validarPr({ head: 'feat/x', base: 'main', titulo: 'feat(ui): algo' }));
  ruim(validarPr({ head: 'develop', base: 'main', titulo: 'feat(ui): algo' }));
  ruim(validarPr({ head: 'main', base: 'develop', titulo: 'chore: sincroniza' }));
  ruim(validarPr({ head: 'hotfix/crash', base: 'develop', titulo: 'fix(audio): evita crash' }));
  ruim(validarPr({ head: 'meu-branch', base: 'develop', titulo: 'feat(ui): algo' }));
  ruim(validarPr({ head: 'feat/x', base: 'release', titulo: 'feat(ui): algo' }));
});

test('versões', () => {
  const v = (a, b, c) => ({ 'Cargo.toml': a, 'package.json': b, 'src-tauri/tauri.conf.json': c });
  ok(validarVersoes({ versoes: v('0.3.0', '0.3.0', '0.3.0'), ultimaTag: '0.2.1' }));
  ruim(validarVersoes({ versoes: v('0.3.0', '0.3.0', '0.2.1'), ultimaTag: '0.2.1' }));
  ruim(validarVersoes({ versoes: v('0.2.1', '0.2.1', '0.2.1'), ultimaTag: '0.2.1' }));
  ruim(validarVersoes({ versoes: v('0.2.0', '0.2.0', '0.2.0'), ultimaTag: '0.2.1' }));
  ruim(validarVersoes({ versoes: v('0.3', '0.3', '0.3') }));
  ok(validarVersoes({ versoes: v('0.10.0', '0.10.0', '0.10.0'), ultimaTag: '0.9.9' }));
  ok(validarVersoes({ versoes: v('0.3.0', '0.3.0', '0.3.0'), modo: 'igual', esperada: '0.3.0' }));
  ruim(validarVersoes({ versoes: v('0.3.0', '0.3.0', '0.3.0'), modo: 'igual', esperada: '0.3.1' }));
});

test('última tag compara números, não texto', () => {
  assert.equal(ultimaTag(['v0.9.0', 'v0.10.0', 'v0.2.1', 'lixo']), '0.10.0');
  assert.equal(ultimaTag([]), null);
});
