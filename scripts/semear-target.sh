#!/usr/bin/env bash
# Copia o target/debug do checkout principal para um worktree novo, para ele não recompilar as
# ~570 dependências (quase todas do Tauri). As crates do projeto são limpas e recompiladas no
# caminho do worktree. Ver docs/contribuicao/compilacao.md.
#
# Uso: scripts/semear-target.sh <caminho-do-worktree>
set -euo pipefail

destino="${1:?uso: scripts/semear-target.sh <caminho-do-worktree>}"
destino="$(cd "$destino" && pwd)"
# o checkout principal é o pai do diretório .git comum a todos os worktrees
principal="$(cd "$(git -C "$destino" rev-parse --path-format=absolute --git-common-dir)/.." && pwd)"

if [[ "$destino" == "$principal" ]]; then
  echo "o destino é o próprio checkout principal" >&2
  exit 1
fi
if [[ ! -d "$principal/target/debug" ]]; then
  echo "sem $principal/target/debug para copiar; compile o checkout principal antes" >&2
  exit 1
fi
if [[ -e "$destino/target" ]]; then
  echo "$destino/target já existe; apague-o antes de semear" >&2
  exit 1
fi

mkdir -p "$destino/target"
cp -a "$principal/target/debug" "$destino/target/"
# o cache incremental só serve às crates do projeto, que vão recompilar de qualquer forma
rm -rf "$destino/target/debug/incremental"

export PATH="$HOME/.cargo/bin:$PATH"
crates=$(cargo metadata --no-deps --format-version 1 --manifest-path "$destino/Cargo.toml" |
  node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>console.log(JSON.parse(s).packages.map(p=>"-p "+p.name).join(" ")))')
# shellcheck disable=SC2086
cargo clean -q --manifest-path "$destino/Cargo.toml" $crates

echo "target semeado em $destino/target ($(du -sh "$destino/target" | cut -f1))"
