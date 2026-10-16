# miupy

Player de música offline, pessoal e de código aberto: discoteca local em SQLite, playlists,
busca e download a partir do YouTube, estatísticas e retrospectiva anual, com visual "magma".
Rust + Tauri 2, para desktop (Linux, Windows, macOS) e Android.

## Estrutura

```
crates/miupy-core    domínio e SQLite
crates/miupy-audio   motor de reprodução
crates/miupy-media   YouTube, metadados e tags
src-tauri/           casca Tauri (comandos IPC)
src/                 frontend Svelte
scripts/             ferramentas (matriz de rastreabilidade)
docs/                requisitos, arquitetura, stacks, relatórios
```

## Desenvolvimento

Pré-requisitos: Rust estável, Node 22+, dependências de sistema do Tauri 2 e `yt-dlp` no `PATH`
(para busca e download).

```sh
npm install
npm run tauri dev        # app completo (ainda não validado; ver docs/relatorios)
npm run dev              # só o frontend, com dados simulados
cargo test --workspace   # testes do núcleo
npm run docs:matriz      # regenera a matriz de rastreabilidade
```

Documentação em [`docs/`](docs/README.md).

## Aviso

Projeto de uso pessoal. Baixar conteúdo do YouTube pode violar os termos de serviço da plataforma
e direitos autorais; use apenas com conteúdo que você tem direito de baixar.

## Licença

Distribuído sob a GNU GPL versão 3 ou posterior (`GPL-3.0-or-later`): [`LICENSE`](LICENSE). Até a
v0.3.1 o projeto era MIT ou Apache 2.0; a troca veio com o `rustypipe` (GPL-3.0), que faz o download
no Android.
