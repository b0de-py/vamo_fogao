# Roteiro de leitura

Uma ordem pra ler o código sem se perder. A ideia é ir de fora pra dentro e depois de dentro pra fora: primeiro o que amarra o projeto, depois o domínio, depois cada adaptador.

1. **`Cargo.toml` da raiz.** O workspace, os metadados que todo crate herda e os lints, cada um com o comentário do porquê. É o contrato do projeto em sessenta linhas.
2. **`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml` e `deny.toml`.** As ferramentas e as regras que a CI cobra.
3. **`crates/vamo_fogao/src/main.rs`.** A porta de entrada do `vamo`. Na fase 0 é pequena; da fase 1 em diante, é onde a config é lida e os adaptadores são ligados ao domínio.
4. **`crates/vamo-core`** (a partir da fase 1). O domínio: tarefas, status, encaixe e as regras. É o crate com mais teste, e os testes ficam no fim de cada arquivo.
5. **Os adaptadores**, um por vez, na ordem em que chegam: `vamo-store` e o import (fase 1), `vamo-tui` (fase 2), `vamo-agenda` (fase 3), `vamo-sync` (fase 5) e `vamo-mcp` (fase 6).
6. **`crates/vamo-acervo`** (a partir da fase 1). O museu de bolso, um domínio à parte.

Duas dicas pra ler com calma:

- Rode `cargo doc --workspace --no-deps --open`. A doc gerada mostra cada crate com os tipos linkados entre si, e muitas vezes é mais fácil navegar por ali do que pelos arquivos.
- Leia os testes depois do código. Eles mostram o comportamento esperado em exemplos pequenos, inclusive os casos de borda que o código trata.
