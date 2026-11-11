# 0007. Desenhar a TUI com o ratatui, recebendo as tarefas por uma porta própria

- **Status:** aceita
- **Data:** 2026-11-11

## Contexto

A fase 2 traz a interface de terminal. Ela precisa ler as tarefas, gravar mudanças (concluir, escalar, editar) e desenhar telas que dê pra testar sem abrir um terminal de verdade. Pela regra da arquitetura (ADR 0002), a TUI é um adaptador e não pode chamar outro adaptador: o `vamo-tui` não pode depender do `vamo-store`.

## Decisão

- A TUI usa o [ratatui](https://ratatui.rs) com o crossterm. Cada quadro é desenhado do zero a partir de um estado (o `App`), e as teclas só mudam o estado.
- A TUI declara a própria porta, a trait `Ledger`: o estado das tarefas, o log e um jeito de gravar mudanças. O binário implementa a `Ledger` com a mesma sessão da CLI (banco, relógio e nome da máquina), e os testes da TUI usam um log em memória.
- As telas são testadas com o `TestBackend` do ratatui e snapshots do insta.

## Alternativas consideradas

- **`vamo-tui` dependendo do `vamo-store`:** mais curto, mas quebra a regra de adaptador não chamar adaptador, e os testes da TUI passariam a precisar de banco.
- **Usar a `EventStore` do core direto na TUI:** a TUI teria que gerar ID de evento, ler o relógio e o nome da máquina, repetindo o que a sessão do binário já faz.
- **cursive ou tui com widgets retidos:** o modelo de estado e desenho do ratatui é o que deixa o teste de snapshot simples e é o padrão do ecossistema hoje.

## Consequências

- O que a TUI faz com as tarefas passa pela mesma gravação da CLI: os eventos saem iguais, venham de onde vierem.
- Regra continua no core. A TUI pergunta ao core se uma mudança vale e só grava o que ele devolve.
- O ratatui traz o foldhash, com a licença Zlib, que entrou na lista do `deny.toml`.
