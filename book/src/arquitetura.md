# Arquitetura

O vamo_fogao segue a arquitetura hexagonal, também chamada de portas e adaptadores. A ideia cabe numa frase: **o domínio não sabe que o mundo de fora existe**. Ele não sabe que as tarefas moram num SQLite, que a agenda vem de um link ICS ou que alguém está olhando por um terminal. Tudo isso fica nas bordas, em adaptadores que traduzem entre o mundo e o domínio.

```text
        quem usa o app                      o mundo de fora
 +--------------------------+        +------------------------------+
 | vamo_fogao  (CLI)        |        | vamo-store   (SQLite)        |
 | vamo-tui    (terminal)   |        | vamo-agenda  (ICS, Google)   |
 | vamo-mcp    (Claude)     |        | vamo-sync    (git, backup)   |
 +------------+-------------+        +--------------+---------------+
              | chama                    implementa |
              v                                     v
        +-------------------------------------------------+
        | vamo-core: tarefas, regras e as portas (traits) |
        +-------------------------------------------------+
```

Do lado esquerdo ficam os adaptadores que **chamam** o domínio: a CLI, a TUI e o MCP recebem um pedido de quem usa e pedem ao core pra fazer. Do lado direito ficam os que **são chamados** por ele: o core declara uma porta, por exemplo "preciso guardar e ler o log de eventos", na forma de uma trait (a `EventStore`), e o `vamo-store` implementa essa trait com SQLite. A agenda é a mesma ideia: o core declara a porta `Agenda`, e o leitor de ICS e o cliente do Google implementam.

## Os crates

| Crate | Papel | Depende de |
|-------|-------|------------|
| `vamo-core` | O domínio: tarefa, status, encaixe, regras do tempo, brechas, título do caderno e as portas. | nenhum crate do workspace |
| `vamo-acervo` | O museu de bolso: fatos, validador e sorteio da curiosidade do dia. | nenhum crate do workspace |
| `vamo-store` | Guarda o log de eventos em SQLite. | `vamo-core` |
| `vamo-agenda` | Lê a agenda (link ICS ou, com a feature `google`, a API do Google) e o rodapé `vamo:` dos eventos. | `vamo-core` |
| `vamo-sync` | Exporta o log de eventos e sincroniza as máquinas pelo git. | `vamo-core` |
| `vamo-tui` | As telas, o tema e o fogo. | `vamo-core`, `vamo-acervo` |
| `vamo-mcp` | O servidor MCP. | `vamo-core`, `vamo-acervo` |
| `vamo_fogao` | O binário `vamo`: lê a config e liga os adaptadores ao core. | todos |

## As regras

- **O domínio não conhece adaptador.** O `vamo-core` não declara dependência de nenhum outro crate do workspace, então um `use vamo_store::...` dentro dele nem compila. A fronteira não depende de disciplina: o Cargo garante.
- **Regra de negócio mora no core.** Adaptador só traduz. Se um adaptador começa a decidir alguma coisa ("essa tarefa está atrasada?"), essa decisão está no lugar errado.
- **Adaptador não chama adaptador.** Quem liga o SQLite ao core, e o core à TUI, é o binário.
- **O core não lê o relógio nem o ambiente.** O instante, o fuso e a config chegam por parâmetro. É o que deixa testar "o que acontece às 23h59 do dia 31" sem esperar o dia 31.

## Por que assim

Porque o app conversa com muita coisa de fora, e as regras que importam precisam de muito teste. Com o domínio isolado, testar uma regra não exige banco, rede nem relógio: o teste roda em milissegundos e dá o mesmo resultado toda vez. E trocar uma peça (o leitor de ICS pela API do Google, por exemplo) vira escrever um adaptador novo, sem mexer em regra nenhuma.

O registro completo da decisão, com as alternativas que ficaram de fora, está na [ADR 0002](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0002-arquitetura-hexagonal-em-workspace.md).
