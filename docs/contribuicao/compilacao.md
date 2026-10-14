# Compilação rápida e sem recompilar à toa

O miupy compila ~570 itens, quase todos do Tauri (359 crates). Este esquema corta o tempo e o
calor da máquina em três frentes: cache entre checkouts (sccache), link rápido (clang + mold) e
um perfil de dev mais leve. Ele também lista o que fazia o `target/` ser jogado fora sem motivo.

## Medições (2026-10-14, 12 threads, `jobs = 6`)

| Situação | Antes | Depois |
|---|---|---|
| Build do zero, cache vazio | 3m54s, 1307 s de CPU | 2m50s, 346 s de CPU |
| Build do zero, mesmo caminho, cache cheio (`cargo clean` e build) | igual ao do zero | 54 s |
| Worktree novo, só sccache | igual ao do zero | 2m00s |
| Worktree novo com `scripts/semear-target.sh` | igual ao do zero | 1,3 s de cópia + 9 s |
| Editar `src-tauri/src/lib.rs` e compilar | — | 1,7 s |
| Binário `target/debug/miupy` | 470 MB | 81 MB |
| `target/` depois de build + test | 11 GB | 3,4 GB |

## As peças

### 1. Um toolchain só (a causa das recompilações completas)

A máquina tem dois Rust 1.99: o pacote `rust` do Arch (`/usr/bin/cargo`) e o do rustup
(`~/.cargo/bin`). O terminal do autor usava o do Arch; os agentes, o do rustup. São builds
diferentes do `rustc`, então **cada troca invalidava o `target/` inteiro**: um `cargo build` logo
depois de um `npx tauri dev` levava quase 4 minutos.

O `~/.zshenv` põe `~/.cargo/bin` na frente do `PATH` em todo shell zsh, interativo ou não. Confira
com `which cargo` (deve dar `~/.cargo/bin/cargo`). O rustup fica como o toolchain oficial, porque
é ele que instala os alvos do Android.

### 2. `~/.cargo/config.toml` (da máquina, fora do repositório)

```toml
[build]
jobs = 6                       # metade dos 12 threads, para não esquentar
rustc-wrapper = "sccache"      # cache em ~/.cache/sccache, compartilhado por todo checkout

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

Fica fora do repositório porque depende do que está instalado nesta máquina (`sccache`, `mold`,
`clang`). Para conferir o linker: `readelf -p .comment target/debug/miupy` deve citar `mold`.

### 3. Perfil de dev (`Cargo.toml` do workspace)

```toml
[profile.dev]
debug = "line-tables-only"     # backtraces com linha; debuginfo bem menor; link mais rápido

[profile.dev.package."*"]
opt-level = 2                  # dependências otimizadas (áudio sem otimização engasga)
debug = false                  # dependências sem debuginfo
```

Se precisar depurar com variáveis no gdb/lldb, troque para `debug = true` só localmente: muda o
perfil e recompila tudo.

### 4. Worktree novo: `scripts/semear-target.sh`

O sccache não basta para worktrees. Crates com *build script* (serde, libsqlite3-sys,
proc-macro2, tauri…) embutem o caminho absoluto do `OUT_DIR`, então num caminho novo elas e tudo
que depende delas erram o cache (74% de acerto, mas os erros são as crates pesadas).
`SCCACHE_BASEDIRS` não resolve isso para Rust (testado).

O script copia o `target/debug` do checkout principal para o worktree, sem o cache incremental, e
limpa só as crates do projeto:

```sh
git worktree add ../miupy-<nome> -b fix/<nome> develop   # ou orca worktree create …
scripts/semear-target.sh ../miupy-<nome>
```

Compile o checkout principal antes, com `cargo build --workspace` e `cargo test --workspace --no-run`
e sem outra compilação em andamento: o script copia o que estiver lá, e um `target/` incompleto
(depois de um `cargo clean -p`, por exemplo) faz o worktree recompilar as crates que faltam. Depois de semear, o
worktree tem o próprio `target/`. **Nunca** aponte `CARGO_TARGET_DIR` de um worktree para o
`target/` de outro: o cargo já reaproveitou um `miupy-core` do checkout errado assim.

## O que causa recompilação (e o que não causa)

| Causa | Efeito | O que fazer |
|---|---|---|
| Trocar de toolchain (`/usr/bin/cargo` ⇄ `~/.cargo/bin/cargo`) | tudo | um toolchain só (peça 1) |
| `RUSTFLAGS` no ambiente | tudo; ele **substitui** o `rustflags` do config, e o mold some | não exporte `RUSTFLAGS`; use o config |
| `CARGO_TARGET_DIR` no ambiente | o sccache errou 100% das crates Rust (medido) | não exporte; um `target/` por checkout. `CARGO_BUILD_JOBS` não afeta o cache (medido) |
| Mudar `[profile.*]` ou `rustflags` | tudo, uma vez | — |
| Primeiro `cargo test` depois de `cargo build` | ~45 crates, uma vez (as dev-dependencies, `tokio/test-util` e `tempfile`, mudam features) | nada: as duas variantes ficam no `target/` |
| Alternar `cargo build` ⇄ `cargo test` | só a crate `miupy`, ~1 s (`info of dependency tauri changed`) | nada |
| `cargo test -p <crate>` em vez de `--workspace` | dependências compartilhadas com outro conjunto de features | prefira `--workspace`; a unificação por workspace (`resolver.feature-unification`) ainda é instável |
| `npx tauri dev` ⇄ `cargo build` | nada (o Tauri CLI roda `cargo run --no-default-features`, mesmas features) | — |
| `cargo clippy` | primeira vez gera os `.rmeta`; depois só as crates do projeto | — |

Para descobrir por que algo recompilou: `cargo build -v 2>&1 | grep Dirty`.

## sccache no dia a dia

- `sccache -s` mostra acertos; `sccache -z` zera as estatísticas.
- *Non-cacheable: crate-type* são proc-macros e build scripts, que o sccache não guarda.
- O cache tem 10 GB por padrão (`SCCACHE_CACHE_SIZE`); ele descarta o mais antigo sozinho.
- **Agentes Codex em sandbox:** o sandbox não escreve em `~/.cache/sccache`. Suba o servidor de
  fora antes de soltar os agentes, para eles só falarem com ele por `127.0.0.1:4226`:
  `SCCACHE_IDLE_TIMEOUT=0 sccache --start-server`. Se algo der errado com o wrapper,
  `RUSTC_WRAPPER= cargo …` compila sem ele.

## Calor

- `jobs = 6` no config; com agentes em paralelo, reparta com `CARGO_BUILD_JOBS` (por exemplo,
  3 + 3) e use `nice`.
- `powerprofilesctl set power-saver` durante compilações longas baixa o teto de clock (não
  aplicado; decisão do autor).
- Rust-analyzer no editor: use um `target/` próprio dele (`"rust-analyzer.cargo.targetDir": true`)
  para não disputar o lock do cargo com o terminal. Ele também precisa achar o `cargo` do rustup.
