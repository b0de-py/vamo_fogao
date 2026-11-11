# 0006. Guardar o log em SQLite, com migrações próprias

- **Status:** aceita
- **Data:** 2026-11-11

## Contexto

O log de eventos (ADR 0004) precisa morar num arquivo na máquina de quem usa, ser lido por mais de um programa ao mesmo tempo (a CLI, e depois a TUI e a barra) e sobreviver a anos de versões novas do app, cada uma podendo mudar o esquema. Quem usa não deveria precisar instalar nada além do `vamo`.

## Decisão

- **SQLite, pelo `rusqlite` com a feature `bundled`.** O SQLite vai compilado dentro do binário, então não depende da versão instalada no sistema. O banco fica em `~/.local/share/vamo_fogao/vamo.db`, ou onde o `XDG_DATA_HOME` apontar.
- **Migrações próprias, em `.sql`.** Cada migração é um arquivo em `crates/vamo-store/migrations/`, embutido no binário com `include_str!`. A versão do esquema é o número que o SQLite guarda no `PRAGMA user_version`: abrir o banco aplica as migrações que faltam, em ordem, cada uma na própria transação, com o `user_version` atualizado dentro dela.
- **Banco de versão mais nova fica intocado.** Se o `user_version` passa do que esta versão do app conhece, o `vamo` recusa abrir, em vez de arriscar estragar dado que ele não entende.
- **Tabela `STRICT`, WAL e espera de 5 segundos.** O `STRICT` faz o SQLite recusar valor do tipo errado; o WAL deixa quem lê e quem escreve trabalharem juntos; o `busy_timeout` espera um pouco quando outro processo está escrevendo.
- **Os valores ficam como o domínio escreve** (`T-041`, `brecha`, `concluída`), pra dar pra abrir o banco no `sqlite3` e entender o que está lá.

## Alternativas consideradas

- **Arquivo JSON ou TOML:** simples, mas dois processos escrevendo ao mesmo tempo corrompem o arquivo, e cada leitura carrega tudo.
- **`sqlx`:** confere as consultas na compilação, mas é assíncrono e traz o tokio pra um app que, até a fase 6, não precisa dele.
- **Diesel ou SeaORM:** um ORM inteiro pra uma tabela de eventos é mais peso que ajuda, e esconde o SQL que o livro quer mostrar.
- **`refinery` ou `rusqlite_migration`:** fazem o mesmo controle de versão, mas o que o projeto precisa cabe em umas quarenta linhas legíveis em `migrations.rs`, sem dependência a mais.

## Consequências

- Migração publicada não muda nunca: mudar o esquema é acrescentar um arquivo novo no fim da lista, com o teste de que um banco antigo chega na versão nova.
- O `vamo` não depende de SQLite no sistema, mas o primeiro build compila o SQLite em C, o que leva alguns segundos a mais.
- Voltar pra uma versão mais velha do app depois de uma migração nova não funciona: o app velho recusa o banco novo, de propósito.
