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
| 2026-10-14 | 05 — contrato da v0.3, capas, áudio, imagens, CI/CD e fluidez (com o prompt da 06) | [sessão](2026-10-14_sessao-05.md) · [capas](2026-10-14_sessao-05_agente-capas.md) · [ci](2026-10-14_sessao-05_agente-ci.md) · [normalizacao](2026-10-14_sessao-05_agente-normalizacao.md) · [imagens](2026-10-14_sessao-05_agente-imagens.md) · [equalizador](2026-10-14_sessao-05_agente-equalizador.md) · [visual](2026-10-14_sessao-05_agente-visual.md) · [skeleton](2026-10-14_sessao-05_agente-skeleton.md) |
| 2026-10-15 | 06 — merges da 05, CI sem bot, favoritas, progresso, yt-dlp e pendências de áudio (com o prompt da 07) | [sessão](2026-10-15_sessao-06.md) · [favoritas](2026-10-14_sessao-06_agente-favoritas.md) · [progresso](2026-10-14_sessao-06_agente-progresso.md) · [ytdlp](2026-10-14_sessao-06_agente-ytdlp.md) · [pendencias-audio](2026-10-14_sessao-06_agente-pendencias-audio.md) |
| 2026-10-15 | 07 — onda 2, pedidos do autor no app real, Surfle e skills (com o prompt da 08) | [sessão](2026-10-15_sessao-07.md) · [busca-vazia](2026-10-15_sessao-07_agente-busca-vazia.md) · [baixar-automatico](2026-10-15_sessao-07_agente-playlist-baixar-automatico.md) · [rede-player](2026-10-15_sessao-07_agente-pendencias-rede-player.md) · [pendencias-ui](2026-10-15_sessao-07_agente-pendencias-ui.md) · [surfle](2026-10-15_sessao-07_agente-surfle.md) · [tocando-layout](2026-10-15_sessao-07_agente-tocando-layout.md) · [dropdown-tema](2026-10-15_sessao-07_agente-dropdown-tema.md) · [discoteca-linhas](2026-10-15_sessao-07_agente-discoteca-linhas.md) |
| 2026-10-15 | 08 — v0.3.0, onda visual 1 e a fluidez do WebKitGTK com GPU (com o prompt da 09) | [sessão](2026-10-15_sessao-08.md) · [metadados](2026-10-15_sessao-07_agente-metadados.md) · [movimento](2026-10-15_sessao-08_agente-movimento.md) |
