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
