# vamo_fogao

App de tarefas em Rust, feito por um botafoguense pra torcida do Botafogo: CLI, TUI, módulo de barra e servidor MCP. O plano está no `ROADMAP.md` e nas issues: cada fase é uma issue épica, com as tarefas como sub-issues.

O Claude escreve o código e a documentação. O Raul revisa os PRs e aprende Rust lendo o código, então clareza vale mais que esperteza: código direto, nomes bons e comentário que explica o porquê (o quê, o código já diz).

## Comandos

- `./scripts/checar.sh`: a mesma checagem da CI. Rode antes de cada commit.
- `cargo run -- <args>`: roda o `vamo`.
- `cargo nextest run --workspace`: os testes. Os exemplos da doc rodam com `cargo test --doc --workspace`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --open`
- `cargo deny check`: licenças, avisos de segurança e fontes das dependências.
- `cargo run -- acervo validar`: a régua do acervo (`acervo/`), a mesma do job Acervo da CI.
- `mdbook serve book --open`: o livro, recarregando a cada arquivo salvo. Mudou a arquitetura, o livro muda no mesmo PR.

## Arquitetura

Hexagonal, num workspace Cargo. As decisões e o porquê de cada uma ficam em `docs/adr/`.

| Crate | Papel | Depende de |
|-------|-------|------------|
| `vamo-core` | Domínio puro: tarefa, status, encaixe, regras do tempo, brechas, título do caderno e as portas (traits) que os adaptadores implementam. | nenhum crate do workspace |
| `vamo-acervo` | O museu de bolso: fatos, validador e sorteio da curiosidade do dia. Tão puro quanto o core. | nenhum crate do workspace |
| `vamo-store` | SQLite, migrações com `PRAGMA user_version`. | `vamo-core` |
| `vamo-agenda` | ICS, Google Agenda (opcional) e o rodapé `vamo:`. | `vamo-core` |
| `vamo-sync` | Log de eventos exportado, sincronia entre máquinas e git. | `vamo-core` |
| `vamo-tui` | Telas, tema e o fogo. | `vamo-core`, `vamo-acervo` |
| `vamo-mcp` | Servidor MCP. | `vamo-core`, `vamo-acervo` |
| `vamo_fogao` | O binário `vamo`: lê a config e liga os adaptadores ao core. | todos |

Regras que não se negociam:

- O domínio não conhece adaptador. Como o `vamo-core` não depende de nenhum outro crate, o próprio Cargo impede o atalho.
- Regra de negócio mora no core. Adaptador só traduz entre o mundo de fora e os tipos do domínio.
- Adaptador não chama adaptador: quem liga um no outro é o binário.
- O core não lê o relógio nem o ambiente. O instante, o fuso e a config chegam por parâmetro.

## Convenções de código

- Nomes do código (tipos, funções, variáveis, testes) em inglês. Documentação, comentários, commits e textos de interface em português. O glossário do livro liga um ao outro: caderno e notebook, brecha e gap, gol e done.
- Erros: um enum com `thiserror` por crate de biblioteca; `color-eyre` só no binário; `?` pra propagar. `unwrap` não passa no clippy fora de teste, e `expect` só com mensagem que diz por que aquilo nunca falha.
- Valor inválido não chega a existir: tipos como `TaskId` e `NotebookTitle` validam na construção, e o resto do código confia neles.
- Parâmetro de função pede `&str` e `&[T]`, não `&String` e `&Vec<T>`. Enum no lugar de `bool` quando o significado não for óbvio na chamada.
- Todo item público tem doc, com `# Errors` e `# Panics` quando couber e exemplo quando ajudar (o exemplo roda como teste).
- `#[allow]` sempre com motivo: `#[allow(clippy::lint, reason = "...")]`.
- Testes de unidade no próprio módulo (`#[cfg(test)] mod tests`), de integração em `tests/`. Parser e projeção ganham teste de propriedade (proptest); saída de tela e de CLI ganha snapshot (insta).
- Dependência nova só se pagar o próprio peso. A versão vai em `[workspace.dependencies]` e o crate pede com `nome.workspace = true`. A licença precisa passar no `cargo deny`.
- A formatação é a do `rustfmt`. O hook em `.claude/settings.json` formata depois de cada edição.

## Fluxo de trabalho

Siga a skill `fluxo-de-fase`. Em resumo:

- Uma branch por fase, `feature/fase-N`, saindo da `main`.
- Um commit por tarefa, que fecha a sub-issue dela com `Closes #N` no rodapé. `./scripts/checar.sh` passando antes do commit e push logo depois.
- Título do commit no imperativo, em português, com até 72 caracteres ("Montar o workspace Cargo com os crates"). O corpo explica o porquê.
- Decisão de arquitetura ganha ADR em `docs/adr/` no mesmo commit.
- No fim da fase, um PR "Fase N: Nome (Codinome)" com um `Closes` pro épico e pra cada sub-issue. O merge é do Raul.
- Nunca: push na `main`, merge de PR, force push, apagar branch ou issue.

## Privacidade

O repositório é público. Nenhum dado pessoal entra no código, nos testes, nos exemplos ou na documentação: nada de tarefa real, evento de agenda, nome de familiar, endereço ou horário da rotina de alguém. Exemplo é sempre inventado. Os dados de verdade ficam na máquina de quem usa e no backup privado.

## Identidade

- As cores são as do r4ul.com.br. Vermelho não entra, nunca: atraso e regra ferida usam amarelo plasma (`#ffff00`).
- Projeto de torcedor: sem escudo e sem marca oficial do clube.
- A interface fala a linguagem de jogo (banco, escalação, gol, súmula), com modo neutro na config.
- Fato do acervo só aparece no app com duas fontes independentes.
