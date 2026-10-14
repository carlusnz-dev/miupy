# Interface dos novos comandos IPC

- **Data:** 2026-10-14
- **Autor:** Codex
- **Modelo:** GPT-6, esforço não informado
- **Escopo:** interface Svelte para os comandos e eventos da sessão 03 (RF-12, RF-18, RF-21 a RF-27).

## O que foi feito

- Foto do perfil com validação JPEG/PNG/WebP até 10 MB, prévia, gravação e remoção; avatar compartilhado com a navegação.
- Capa de playlist escolhida pelo usuário e componente de capa com mosaico padrão nas listas e no detalhe.
- Sugestões do YouTube com debounce de 200 ms, navegação por teclado, semântica combobox/listbox e descarte de respostas antigas.
- Botão de aleatório em barra e tela Tocando, com os modos `off`, `random` e `smart` refletindo `PlayerState.shuffle`.
- Pontuação discreta nas linhas de faixa e na tela Tocando.
- Rota DNA com janelas de 7, 30, 90 e 365 dias, indicadores, artistas, gêneros e tendências; estado vazio inclui corretamente perfis derivados só de favoritas.
- Indicador de carregamento de streaming e toast para `player://error`.
- Foto e crédito do artista.
- Configurações de yt-dlp e reprocessamento de metadados, incluindo os eventos de progresso.
- Cliente real, mock e tipos atualizados; README da interface e matriz de rastreabilidade regenerados.

## Decisões

- O mosaico repete imagens em grades de quatro células quando há duas ou três imagens, mantendo a composição quadrada.
- Pontuação aparece como número pequeno com tooltip que explica plays, pulos e última reprodução.
- A tela DNA considera um perfil com artistas/gêneros mesmo que o tempo ouvido seja zero, como ocorre com favoritas sem reprodução.
- O endpoint de avatar e capa espera `ImageUpload` em memória; no mock, os dados são URLs locais `data:` para permitir prévia no navegador.
- O contrato não indica progresso para `ytdlp_update` além de `ytdlp://status`; a UI usa esse evento e não inventa campos.

## Verificação (comandos rodados e resultado real)

| Comando | Resultado |
|---|---|
| `npm run check` | 0 erros e 0 avisos |
| `nice -n 10 npm test` | 3 arquivos, 12 testes passando |
| `nice -n 10 npm run build` | passou; 171 módulos transformados |
| `rg -n 'createMockApi|buildTracks|mockArt' dist` | nenhuma ocorrência; mock fora do bundle de produção |
| `npm run docs:matriz` | 37 requisitos: 27 verificados, 6 implementados, 4 pendentes |
| `curl -fsS http://127.0.0.1:1420/` | HTML da aplicação servido com sucesso |
| `git diff --check` | sem erros de whitespace |

Não rodei cargo conforme o escopo do briefing. Iniciei `npm run dev -- --host 127.0.0.1`; Vite informou que a porta wildcard 1420 já estava ocupada, mas serviu em 127.0.0.1:1420. Não há navegador ou ferramenta de console nesta sessão: não vi as telas e não confirmei ausência de erros de console. O servidor HTTP respondeu.

## O que foi verificado rodando

- Vitest para debounce e descarte de respostas atrasadas, ciclo de modos, upload/tamanho/tipo de imagem, arranjos do mosaico e concorrência na troca de janela do DNA.
- Check de tipos Svelte, build de produção e requisição HTTP ao servidor de desenvolvimento.
- O bundle produzido não contém identificadores do cliente falso nem da geração de faixas do mock.

## O que só foi escrito

- As interações visuais e acessibilidade por teclado não foram exercitadas em navegador.
- A integração com os comandos reais depende da casca Rust ainda não implementada.

## Pendências e problemas conhecidos

- A tela de DNA e os estados de erro/progresso precisam de inspeção visual quando houver janela disponível.
- A troca de capa do playlist foi implementada no detalhe; o mosaico reaproveitável está na lista e no detalhe.

## Próximos passos

- Integrar com a casca Rust seguindo `docs/arquitetura/ipc.md`.
- Abrir o app no navegador/Tauri e conferir console, teclado e estados de progresso.
