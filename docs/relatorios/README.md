# Relatórios de sessão

Um arquivo `.md` por sessão de trabalho, salvo ao fim dela. Agentes que participam de uma sessão
salvam o próprio relatório, e o relatório da sessão aponta para eles.

## Nome do arquivo

- Sessão: `AAAA-MM-DD_sessao-NN.md`
- Agente: `AAAA-MM-DD_sessao-NN_agente-<nome>.md`

## Modelo

```markdown
# <título curto>

- **Data:** AAAA-MM-DD
- **Autor:** <quem conduziu>
- **Modelo:** <modelo e esforço>
- **Escopo:** <uma linha>

## O que foi feito
## Decisões
## Verificação (comandos rodados e resultado real)
## Pendências e problemas conhecidos
## Próximos passos
```

Regra: a seção de verificação traz o resultado real dos comandos. Se algo não foi rodado ou
falhou, isso fica escrito.

## Índice

| Data | Sessão | Arquivo |
|---|---|---|
| 2026-10-13 | 01 — fundação do projeto | [sessão](2026-10-13_sessao-01.md) · [core](2026-10-13_sessao-01_agente-core.md) · [audio](2026-10-13_sessao-01_agente-audio.md) · [media](2026-10-13_sessao-01_agente-media.md) · [frontend](2026-10-13_sessao-01_agente-frontend.md) |
| 2026-10-13 | 02 — YouTube real, streaming, recomendações e fluxo de git | [sessão](2026-10-13_sessao-02.md) · [core](2026-10-13_sessao-02_agente-core.md) · [audio](2026-10-13_sessao-02_agente-audio.md) · [media](2026-10-13_sessao-02_agente-media.md) · [frontend](2026-10-13_sessao-02_agente-frontend.md) · [shell](2026-10-13_sessao-02_agente-shell.md) |
| 2026-10-14 | 03 — DNA, aleatório, imagens, yt-dlp do app e transições | [sessão](2026-10-14_sessao-03.md) · [core](2026-10-14_sessao-03_agente-core.md) · [media](2026-10-14_sessao-03_agente-media.md) · [ui-telas](2026-10-14_sessao-03_agente-ui-telas.md) · [ui-movimento](2026-10-14_sessao-03_agente-ui-movimento.md) · [shell](2026-10-14_sessao-03_agente-shell.md) |
| 2026-10-14 | 04 — compilação rápida, v0.2.0 e pendências da 03 (com o prompt da 05) | [sessão](2026-10-14_sessao-04.md) · [pausa](2026-10-14_sessao-04_agente-pausa.md) · [limite](2026-10-14_sessao-04_agente-limite.md) · [gpu](2026-10-14_sessao-04_agente-gpu.md) · [aleatorio](2026-10-14_sessao-04_agente-aleatorio.md) |
| 2026-10-14 | 05 — correção de capas | [capas](2026-10-14_sessao-05_agente-capas.md) |
| 2026-10-14 | 05 — integração contínua | [ci](2026-10-14_sessao-05_agente-ci.md) |
| 2026-10-14 | 05 — correção de capas e visual | [capas](2026-10-14_sessao-05_agente-capas.md) · [visual](2026-10-14_sessao-05_agente-visual.md) |
