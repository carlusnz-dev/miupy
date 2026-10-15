---
name: relatorio
description: Escreve o relatório de agente ou de sessão do miupy em docs/relatorios/ a partir do git log e das verificações rodadas. Use no fim de toda frente ou sessão.
---

# Relatório

Modelo e nomes em `docs/relatorios/README.md` (não o reescreva aqui; leia só o bloco "Modelo").

- Agente: `docs/relatorios/AAAA-MM-DD_sessao-NN_agente-<frente>.md`.
- Sessão: `docs/relatorios/AAAA-MM-DD_sessao-NN.md`; só a orquestração edita o índice do README.

## Como montar sem gastar

1. Fatos do git, não da memória:
   ```sh
   git log --oneline develop..HEAD
   git diff --stat develop...HEAD
   ```
2. **O que foi feito:** um item por commit lógico, dizendo o comportamento (não a lista de arquivos).
3. **Decisões:** só as que fogem do contrato (`ipc.md`, requisitos) ou que outro agente precisa
   saber. Uma linha cada, com o motivo.
4. **Verificação:** cole o resultado real (contagens de `test result`, `Tests N passed`, erros).
   O que não rodou fica escrito como "não rodado" com o motivo. Teste `#[ignore]` não "passou".
5. **Pendências:** o que ficou sem fazer, sem provar (rede real, app real, DOM) ou com risco.
6. **Próximos passos:** só se houver; no relatório de sessão, o prompt da próxima sessão vai no
   fim, num bloco `text`.

Curto: o relatório de agente cabe em 40 a 70 linhas. Sem repetir o briefing.
