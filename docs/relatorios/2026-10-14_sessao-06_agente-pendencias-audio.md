# Pendências do áudio (EQ e normalização)

- **Data:** 2026-10-14
- **Autor:** agente pendencias-audio
- **Modelo:** Claude Sonnet 5.5
- **Escopo:** pendências da revisão do equalizador (#36, RF-28/RF-29): reset sem alocação, ganhos do EQ consistentes e testes do CRUD de perfis.

## O que foi feito

- `Equalizer::reset` só zera `z1`/`z2` dos biquads em buffers já alocados e conclui uma
  transição em andamento (troca `current`/`next` sem alocar). Antes recriava a struct e
  recalibrava (`calibrated`) na thread de áudio a cada seek. Teste: capacidade e ponteiros dos
  buffers intactos, estados zerados, silêncio na saída.
- `DspControls.equalizer` virou seqlock: `eq_version` (ímpar durante a escrita), escritores
  serializados por um `Mutex` que a thread de áudio nunca toma, `read_equalizer()` sem lock e sem
  alocação (até 8 tentativas; se falhar mantém os ganhos anteriores). Teste com escritor e leitor
  concorrentes: nunca vê mistura.
- Seis testes novos do CRUD de perfis (núcleo): nome repetido ignorando caixa, limite de 30
  caracteres (o contrato diz 1 a 30; 31 é erro), editar mantém id, ordem da lista, máximo de 3
  (e vaga liberada após apagar), apagar o ativo volta para `flat`, apagar outro mantém a seleção.

## Decisões

- O briefing citava 31 caracteres como o limite; o contrato (`ipc.md`) é 30, então 30 passa e 31 falha.
- Nenhum defeito apareceu no CRUD; não houve commit `fix` no núcleo.

## Verificação (comandos rodados e resultado real)

- `cargo fmt --all --check`: ok.
- `cargo clippy --workspace --all-targets -- -D warnings`: ok (uma falha de `collapsible_if` achada e corrigida antes).
- `cargo test --workspace`: tudo verde (core 88, audio 35 + 1 ignorado, shell 72, demais ok).
- `node scripts/rastreabilidade.mjs --check`: 50 requisitos, nenhum ID desconhecido.
- `git diff --check`: limpo.
- Frontend não tocado; `npm` não rodado.

## Pendências e problemas conhecidos

- O seek ainda zera o histórico do limitador com `fill`, sem alocar; não há contador de alocações
  em teste (a prova é por capacidade/ponteiro dos buffers).

## Próximos passos

- Nenhum específico.
