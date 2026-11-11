# 0002. Arquitetura hexagonal num workspace Cargo

- **Status:** aceita
- **Data:** 2026-11-11

## Contexto

O vamo_fogao conversa com muita coisa de fora: um banco SQLite, a agenda (por ICS ou pela API do Google), o terminal, a barra do desktop, as notificações, o git e o Claude por MCP. As regras que importam (status de tarefa, encaixe, regras do tempo, brechas, título no padrão do caderno) precisam de muito teste e não podem depender de nenhuma dessas coisas pra serem testadas.

## Decisão

Arquitetura hexagonal (portas e adaptadores), com a fronteira desenhada pelo próprio Cargo:

- `vamo-core` é o domínio. Não faz IO, não lê o relógio nem o ambiente, e define como traits as portas que precisa (armazenamento, agenda).
- `vamo-acervo` é um segundo domínio, o museu de bolso, tão puro quanto o core.
- `vamo-store`, `vamo-agenda`, `vamo-sync`, `vamo-tui` e `vamo-mcp` são adaptadores: dependem do domínio e traduzem entre ele e o mundo.
- `vamo_fogao` é o binário `vamo`. Lê a config, escolhe os adaptadores e liga tudo.

Cada parte é um crate num workspace só. Como o `vamo-core` não declara dependência de nenhum crate do workspace, importar um adaptador dentro do domínio nem compila.

## Alternativas consideradas

- **Um crate só, com módulos:** mais simples no começo, mas a fronteira vira convenção, e convenção se quebra num dia de pressa. Também recompila tudo a cada mudança.
- **Camadas clássicas (controller, service, repository):** o domínio acaba dependendo do banco, e testar regra passa a exigir banco.
- **Plugins carregados em tempo de execução:** flexibilidade que ninguém pediu, ao custo de `unsafe` ou de uma camada de serialização.

## Consequências

- A regra de dependência é garantida pelo compilador, não pela revisão.
- O domínio é testado sem banco, sem rede e sem relógio, rápido e determinístico.
- Trocar ICS por OAuth, ou SQLite por outra coisa, é escrever um adaptador novo sem mexer nas regras.
- Há mais `Cargo.toml` pra cuidar, e uma funcionalidade pequena às vezes atravessa dois ou três crates.
- O build incremental fica mais rápido: mexer na TUI não recompila o domínio.
