#!/usr/bin/env bash
# Roda na sua máquina a mesma checagem da CI, na mesma ordem.
# Se passar aqui, passa lá.
#
# Uso: ./scripts/checar.sh
set -euo pipefail
cd "$(dirname "$0")/.."

passo() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }

precisa() {
    if ! cargo "$1" --version > /dev/null 2>&1; then
        printf 'Falta o cargo-%s. Instale com: cargo install --locked cargo-%s\n' "$1" "$1" >&2
        exit 1
    fi
}

precisa nextest

passo "Formatação"
cargo fmt --all --check

passo "Clippy"
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

passo "Testes"
cargo nextest run --workspace --all-features --locked
cargo test --doc --workspace --all-features --locked

passo "Documentação"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked

printf '\n\033[1;32mTudo certo. Pode subir.\033[0m\n'
