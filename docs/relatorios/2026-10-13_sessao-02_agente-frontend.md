# Discoteca, streaming e recomendações no frontend

- **Data:** 2026-10-13
- **Autor:** frontend
- **Modelo:** gpt-6-sol (esforço medium), via `codex exec`
- **Escopo:** `src/`, documentação do frontend e dependências JS

## O que foi feito

- Troquei os textos visíveis de Biblioteca para Discoteca, sem alterar identificadores, rotas ou comandos IPC. Atualizei a descrição inicial do README e a documentação de `src/`.
- Ampliei tipos e clientes IPC conforme `ipc.md`: faixas por artista, fila completa, anos da retrospectiva, limpeza de downloads, perfil de gosto, recomendações, evento de recomendações e os campos de gênero, ano e streaming. As telas do artista, fila, retrospectiva e downloads usam os comandos novos.
- Habilitei a indicação de streaming e de conexão nas linhas e no player, mantendo o download como ação própria. Erro `network` ao tocar mostra “Sem conexão: baixe a faixa para ouvir offline.”
- Criei a seção “Para você” no início e sugestões contextualizadas por `playlistId` no detalhe da playlist. Elas mostram capa, motivo e ações de adicionar ou dispensar; atualizar apresenta carregamento e erro de rede. Estatísticas exibem o perfil de gosto; tocando agora exibe gêneros e ano.
- Adicionei movimento de mola reutilizável para páginas e toasts, entrada escalonada em listas, resposta a hover/press e bounce em ações, transições do modal e do mini player, e fade de capas. As transições novas usam transform e opacity e respeitam redução de movimento.
- Passei a importar o mock apenas em desenvolvimento com `import.meta.env.DEV`. Instalei Vitest e criei testes dos stores player, library, downloads e recommendations com API falsa.

## Decisões

- A rota `#/biblioteca` e os nomes de código `library` permanecem para preservar links e o contrato. Na interface o nome é Discoteca.
- O mock mantém dados fictícios e simula a transição `loading` → `playing` em streaming; não grava arquivos ao tocar.
- Licença, fonte de biografia/foto de artista e distribuição de yt-dlp ficaram em aberto. Esta frente não escolheu nenhuma delas.

## Verificação (comandos rodados e resultado real)

- `npm ci`: passou; 64 pacotes instalados, zero vulnerabilidades reportadas.
- `npm install -D vitest`: passou; 15 pacotes adicionados, zero vulnerabilidades reportadas.
- `npm run check`: passou, 0 erros e 0 avisos.
- `npm test`: passou, 1 arquivo e 5 testes. Os testes cobrem play/pause/fila/streaming e erro de rede, eventos do player e da discoteca, progresso e limpeza de download, adição/dispensa/evento/erro de rede de recomendações.
- `npm run build`: passou; Vite transformou 161 módulos e gerou `dist/`.
- Inspeção por script dos quatro arquivos de `dist/`: zero ocorrências dos marcadores exclusivos do mock `yt-n01`, `yt-n08`, `Duo de Belo Horizonte que mistura`, `createMockApi` e `buildTracks`. Os nomes “Aurora Basalto” e “Cratera de Neon” aparecem no build por serem também textos da prévia visual em `Settings.svelte`; por isso não servem como prova de inclusão do mock.
- `git diff --check`: passou, sem erros.
- `git add -A && git commit -m "Implementa discoteca, streaming e recomendações no frontend"`: falhou antes do commit, ao criar `/home/cabeto/Documentos/Projetos/miupy/.git/worktrees/s02-frontend/index.lock`: `Read-only file system` (exit code 128). Não houve commit.
- `cargo fmt --all`, cargo clippy e cargo test: não rodados, conforme a instrução desta frente para não executar cargo nem compilar Rust.
- `npm run docs:matriz`: não rodado; reservado à orquestração conforme instrução desta frente.

## Pendências e problemas conhecidos

- Sem navegador ou janela disponível: o visual e as animações **não foram vistos**; foram escritos, verificados pelo compilador e incluídos no build, mas não avaliados visualmente.
- A integração real com os comandos novos do backend depende da conclusão das outras frentes; os testes automatizados usaram cliente IPC falso.
- As alterações e este relatório permanecem sem commit porque o diretório Git do worktree recusou a criação de `index.lock`. A falha de permissão não foi contornada.

## Próximos passos

- Abrir o app e conferir: entradas/saídas de páginas, listas, modal, toasts e mini player; hover/press/bounce; capas e troca de conteúdo; redução de movimento; leitura dos motivos das recomendações; estados vazio/carregando/sem rede; streaming de faixa não baixada, erro offline e download separado; responsividade desktop e Android.
- Corrigir a permissão de escrita do diretório Git do worktree e então fazer `git add -A` e o commit neste branch.
- Executar a matriz de rastreabilidade e as verificações Rust na orquestração da sessão.

## Nota da orquestração (revisão antes do merge)

O commit foi feito pela orquestração: o sandbox do agente não deixa escrever no diretório git do
worktree. Na revisão foram corrigidos o modelo citado no cabeçalho, uma troca indevida de
"biblioteca" por "discoteca" num comentário sobre biblioteca de software (`Icon.svelte`) e uma
anotação `@req RNF-09` que não correspondia ao código. `npm run check`, `npm test` e
`npm run build` foram rodados de novo pela orquestração, com o mesmo resultado.
