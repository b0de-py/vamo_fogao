#!/usr/bin/env bash
# Hook do Claude Code (PostToolUse): depois de cada edição, formata o
# workspace. O `cargo fmt` é rápido e idempotente, então rodar sempre sai
# mais simples e mais robusto do que descobrir qual arquivo mudou.
#
# Se o arquivo editado ainda não compila, o rustfmt falha e o hook segue
# calado: quem avisa do erro é o compilador, no próximo `cargo`.
set -euo pipefail

# O hook recebe um JSON no stdin. Aqui ele não é necessário.
cat > /dev/null

cd "${CLAUDE_PROJECT_DIR:-.}"
cargo fmt --all --quiet 2> /dev/null || true
