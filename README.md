# vamo_fogao

Um app de tarefas pro terminal, feito por um botafoguense pra torcida do Botafogo.

[![CI](https://github.com/b0de-py/vamo_fogao/actions/workflows/ci.yml/badge.svg)](https://github.com/b0de-py/vamo_fogao/actions/workflows/ci.yml)

## O que vai ser

O vamo_fogao junta num lugar só as tarefas, a agenda e um pedaço da história do clube.

- **Tarefas que cabem no caderno.** Todo título tem até cinco palavras e começa com verbo, do jeito que cabe numa linha de caderno. O app é o lugar principal, e o caderno de papel vira a cópia visual do dia.
- **CLI e TUI.** Comandos rápidos no terminal e uma interface que aparece e some num atalho do Hyprland.
- **Agenda.** Lê a agenda pelo link ICS (com OAuth do Google como opção), acha as brechas do expediente e liga evento a tarefa por uma linha no fim do evento.
- **Linguagem de jogo.** Concluir é gol, as tarefas do dia são a escalação e o resumo do dia é a súmula. Quem preferir liga o modo neutro.
- **Curiosidade do dia.** Uma história do Botafogo por dia, tirada de um acervo em que cada fato tem duas fontes independentes. A zoeira mora separada, na Resenha, e não se mistura com a história.
- **Fogo alvinegro.** Um fogo em preto e branco no rodapé da tela, que sobe com o placar do dia.
- **Claude por MCP.** Um servidor MCP pro Claude ler o dia, agendar tarefas e fazer a revisão semanal gastando pouco contexto.

## Em que pé está

Fase 0, a fundação: workspace, CI, regras de qualidade, ADRs e o esqueleto do livro. Ainda não dá pra usar no dia a dia; os comandos de tarefa chegam na fase 1. O [ROADMAP](ROADMAP.md) mostra todas as fases.

## Como compilar

Precisa só do [rustup](https://rustup.rs). A versão do Rust vem do `rust-toolchain.toml` e é instalada sozinha.

```sh
git clone https://github.com/b0de-py/vamo_fogao
cd vamo_fogao
cargo run -- --help
```

Pra rodar na sua máquina a mesma checagem da CI, use `./scripts/checar.sh`. Ele precisa do `cargo-nextest`, do `cargo-deny` e do `mdbook`, e diz como instalar o que faltar.

## Como o projeto anda

- Cada fase é uma issue épica com as tarefas como sub-issues, numa branch própria, com um commit por tarefa e um PR no fim.
- As decisões de arquitetura ficam em [docs/adr](docs/adr/), cada uma com o contexto e as alternativas que ficaram de fora.
- O livro explica o código pra quem quer ler: `mdbook serve book --open`, ou direto pelos arquivos em [book/src](book/src/introducao.md).
- O código é escrito com o [Claude Code](https://claude.com/claude-code), seguindo as regras do [CLAUDE.md](CLAUDE.md), e cada PR passa pela revisão de um humano.

## Contribuir

Por enquanto, o melhor jeito de ajudar é abrir uma issue com ideia ou bug. Na fase 7, o acervo abre pra torcida sugerir curiosidades, sempre com a régua das duas fontes.

## Projeto de torcedor

O vamo_fogao é feito por torcedor, sem nenhum vínculo com o Botafogo de Futebol e Regatas, e não usa escudo nem marca oficial do clube.

## Licença

O código é distribuído sob a licença MIT ou Apache-2.0, à sua escolha ([LICENSE-MIT](LICENSE-MIT) e [LICENSE-APACHE](LICENSE-APACHE)). Os textos do acervo, quando chegarem, ficam sob a CC BY-SA 4.0. O porquê está na [ADR 0003](docs/adr/0003-licencas.md).

Contribuições entram sob as mesmas licenças.
