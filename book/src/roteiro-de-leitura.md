# Roteiro de leitura

Uma ordem pra ler o código sem se perder. A ideia é ir de fora pra dentro e depois de dentro pra fora: primeiro o que amarra o projeto, depois o domínio, depois cada adaptador.

1. **`Cargo.toml` da raiz.** O workspace, os metadados que todo crate herda e os lints, cada um com o comentário do porquê. É o contrato do projeto, numa página.
2. **`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml` e `deny.toml`.** As ferramentas e as regras que a CI cobra.
3. **`crates/vamo-core`.** O domínio: tarefas, status, encaixe, o título do caderno e o log de eventos. Comece pelo `lib.rs`, que apresenta o crate, e siga pelo capítulo [O domínio](o-dominio.md). É o crate com mais teste, e os testes ficam no fim de cada arquivo.
4. **`crates/vamo-store`.** O primeiro adaptador: o log de eventos num SQLite, com as migrações em `.sql`. O capítulo [O log de eventos](o-log-de-eventos.md) liga ele ao core.
5. **`crates/vamo_fogao/src/`.** O binário `vamo`: `main.rs` liga tudo, `cli.rs` descreve a linha de comando, `commands.rs` roda cada comando, `config.rs` lê o arquivo de config, `import.rs` traz o banco antigo e `acervo.rs` roda o validador do acervo.
6. **`crates/vamo-acervo`.** O museu de bolso, um domínio à parte, com a régua das fontes. O capítulo [O acervo](o-acervo.md) mostra como ele lê cada arquivo.
7. **`crates/vamo-tui`.** As telas do terminal. O `voice.rs` é o dicionário: todo texto de interface, da TUI e da CLI, na linguagem de jogo e na neutra.
8. **Os próximos adaptadores**, na ordem em que chegam: `vamo-agenda` (fase 3), `vamo-sync` (fase 5) e `vamo-mcp` (fase 6).

Duas dicas pra ler com calma:

- Rode `cargo doc --workspace --no-deps --open`. A doc gerada mostra cada crate com os tipos linkados entre si, e muitas vezes é mais fácil navegar por ali do que pelos arquivos.
- Leia os testes depois do código. Eles mostram o comportamento esperado em exemplos pequenos, inclusive os casos de borda que o código trata.
