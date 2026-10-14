# Movimento compartilhado e modal

- **Data:** 2026-10-14
- **Autor:** Carlos Antunes
- **Modelo:** Codex GPT-6
- **Escopo:** presença de elementos entre telas e animação centralizada dos modais (RNF-10).

## O que foi feito

- Criada uma ação de presença compartilhada identificada por chave. Marca só elementos escolhidos,
  captura seus retângulos e anima o FLIP com `transform`; uma nova navegação substitui a captura e
  cancela animações em andamento. Com `prefers-reduced-motion`, usa fade curto.
- Marcadas capa e título da playlist entre lista e detalhe, capa/título/artista nos players e em
  “Tocando”, e nome do artista em linhas de faixa e no cabeçalho do artista.
- Modal agora tem `margin: auto`, abertura com escala e opacidade pelo centro e fechamento animado
  junto ao backdrop; Escape e clique fora também preservam a animação.
- Adicionados testes de cálculo FLIP, registro/limpeza de chaves e substituição de captura em
  navegação interrompida. Atualizados README do frontend e matriz de rastreabilidade.

## Decisões

- Não foi adicionada dependência. O FLIP próprio lê apenas os nós explicitamente registrados e usa
  Web Animations API com propriedades de composição (`transform` e `opacity`).
- A View Transitions API não foi conectada: o roteador atual atualiza `router.current` dentro do
  listener de `hashchange`, depois que o navegador já mudou o hash. Iniciar uma view transition
  nesse ponto não envolveria de forma confiável a atualização SPA; interceptar links exigiria mudar
  o roteador, fora dos arquivos autorizados. O FLIP funciona como fallback no WebKitGTK e Android,
  sujeito à disponibilidade da Web Animations API.
- A causa do modal no canto era a ausência de `margin: auto` em `dialog.modal`, apesar do reset de
  margens do preflight. A regra de centralização foi adicionada.
- O contrato IPC não mudou.

## Verificação (comandos rodados e resultado real)

| Comando | Resultado |
|---|---|
| `npm run check` | 0 erros, 0 avisos |
| `nice -n 10 npm test` | 4 arquivos, 17 testes passando |
| `nice -n 10 npm run build` | build Vite concluído; bundle JS de 174,96 kB (56,42 kB gzip) |
| `npm run docs:matriz` | 37 requisitos: 28 verificados, 6 implementados, 3 pendentes |
| `npm run dev -- --host 127.0.0.1` + `curl http://127.0.0.1:1420/` | servidor Vite pronto; página HTML pt-BR respondeu |

Não rodei comandos Cargo: o escopo é só frontend e o briefing desta frente dispensa a verificação
Rust.

## Verificado rodando e escrito

- **Verificado rodando:** compilador Svelte, testes da matemática FLIP e snapshots, build, geração
  da matriz e resposta HTTP do Vite em modo mock.
- **Só escrito, não observado em janela:** transições entre telas, fechamento do modal, layouts
  desktop/mobile e execução em WebKitGTK/Android. Não havia janela de navegador acessível nesta
  sessão; o autor precisa conferir esses fluxos no app.

## Pendências e problemas conhecidos

- A View Transitions API de mesmo documento ainda não é usada; o motivo e a restrição do roteador
  atual estão descritos em Decisões.
- As fotos de artista não têm origem compartilhada disponível nas telas que apenas exibem uma faixa;
  o nome do artista está marcado, a foto só está identificada no cabeçalho do artista.
- O Vite avisou que a porta 1420 estava ocupada em endereço wildcard; o host `127.0.0.1` serviu
  normalmente. A resposta indica outro servidor ativo nessa porta.

## Próximos passos

- Abrir o app no WebKitGTK e Android e validar animação, alinhamento do modal e navegação rápida,
  inclusive voltar/avançar pelo hash do navegador.
