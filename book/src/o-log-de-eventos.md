# O log de eventos

O vamo_fogao não guarda o estado das tarefas. Guarda o que aconteceu com elas, um evento por mudança, e calcula o estado lendo o log do começo ao fim. Assim o histórico de cada tarefa vem de graça (é a súmula do `vamo show`), e dois PCs conseguem juntar o que cada um fez sem perder nada. A decisão e as alternativas estão na [ADR 0004](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0004-log-de-eventos-com-ultimo-que-escreveu-ganha.md).

## O evento

Em `crates/vamo-core/src/event.rs`. Um `Event` tem cinco campos:

| Campo | Tipo | O que é |
|-------|------|---------|
| `id` | `EventId` | Um UUID da versão 7, que começa pelo instante em que foi gerado. Nunca se repete. |
| `at` | `jiff::Timestamp` | O instante da mudança, em UTC. |
| `machine` | `Machine` | O nome do PC onde a mudança aconteceu. |
| `task` | `TaskId` | A tarefa que mudou. |
| `change` | `Change` | O que mudou. |

`Change` é um enum com dados: cada variante carrega o valor novo do campo que mudou (`Title`, `Area`, `Fit`, `Status`, `Notes` e `Day`, o dia da escalação), e `Created` carrega os quatro campos de uma vez, porque uma tarefa nasce com todos eles. `Check` marca uma recorrente num dia, e cada dia conta como um campo à parte: marcar hoje não mexe em ontem.

O core só confere o UUID (`EventId::new` recusa o que não for da versão 7). Quem gera é o binário, com `Uuid::now_v7()`, porque gerar um ID desses lê o relógio e sorteia bits, e o core não lê relógio.

## A projeção

Em `crates/vamo-core/src/projection.rs`. A `Projection` é o estado das tarefas calculado a partir do log. Ela aplica cada evento com uma regra só: **o último que escreveu ganha, campo por campo**.

"Último" é decidido pelo carimbo do evento:

```rust,ignore
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Stamp {
    at: Timestamp,
    id: EventId,
}
```

O `derive(Ord)` compara os campos na ordem em que estão declarados: primeiro o instante e, no empate, o ID. Como dois eventos nunca têm o mesmo ID, nunca há empate de verdade, e a ordem em que os eventos chegam deixa de importar. Cada campo da tarefa guarda o valor e o carimbo de quem escreveu por último; um evento só escreve se o carimbo dele for maior.

Campo por campo quer dizer: se o notebook mudou o título e o PC do trampo mudou o status, as duas mudanças ficam. Só quando os dois mexem no mesmo campo é que o mais recente ganha.

A `Projection` implementa `FromIterator<&Event>`, então montar o estado a partir do log é um `collect`:

```rust,ignore
let projection: Projection = events.iter().collect();
```

### Três propriedades, testadas com proptest

As garantias da projeção são exatamente as que a sincronia entre PCs vai precisar, então elas têm teste de propriedade: o proptest gera logs aleatórios e confere que

1. **a ordem de chegada não importa**: o mesmo conjunto de eventos, embaralhado, dá o mesmo estado;
2. **receber duas vezes não muda nada**: aplicar um evento repetido não mexe no estado;
3. **duas máquinas que trocam eventos chegam ao mesmo estado**, não importa o que cada uma tinha antes.

## A porta e o SQLite

O core precisa guardar e ler o log, mas não sabe onde. Ele declara uma porta, em `crates/vamo-core/src/port.rs`:

```rust,ignore
pub trait EventStore {
    type Error: std::error::Error + Send + Sync + 'static;

    fn append(&mut self, events: &[Event]) -> Result<usize, Self::Error>;
    fn events(&self) -> Result<Vec<Event>, Self::Error>;
}
```

O tipo associado `Error` deixa cada adaptador trazer o seu próprio erro, sem o core precisar conhecer erro de SQLite.

O `vamo-store` implementa a porta em `crates/vamo-store/src/lib.rs`. Os detalhes que importam:

- **Uma tabela, `events`,** com uma linha por evento e as colunas do valor que mudou. É `STRICT`, então o SQLite recusa valor do tipo errado, e um `CHECK` só aceita os tipos de mudança conhecidos.
- **O mesmo evento entra uma vez só.** O ID é `UNIQUE`, e a gravação usa `INSERT OR IGNORE`. Juntar o log de outro PC vira gravar tudo e deixar o banco descartar o repetido.
- **Ou entram todos, ou nenhum.** O `append` grava numa transação.
- **O esquema evolui por migrações** em `crates/vamo-store/migrations/`, aplicadas em ordem pelo número que o SQLite guarda no `PRAGMA user_version`. A 0002, por exemplo, refez a tabela pra aceitar `day` e `check`, copiando os eventos antigos na mesma ordem. O porquê de não usar uma biblioteca de migração está na [ADR 0006](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0006-sqlite-com-migracoes-proprias.md).
- **Ler valida de novo.** Cada valor lido do banco passa pelo `FromStr` do tipo do domínio. Se alguém editar o banco na mão e escrever uma área inválida, o erro diz a coluna e o motivo, em vez de uma tarefa quebrada aparecer na tela.

## O caminho de um `vamo done`

Pra ver as peças juntas, siga um `vamo done 41` pelo código:

1. **`cli.rs`:** o clap lê os argumentos e entrega um `Command::Done { id }`, com o `TaskId` já validado.
2. **`commands.rs`, `Session::open`:** abre o banco, lê o log inteiro e monta a projeção.
3. **`Projection::status_change`:** pergunta ao core se a tarefa existe e se a transição pra concluída vale. A regra é do core; a resposta é a `Change` a gravar ou o erro.
4. **`Session::record`:** carimba o evento (UUID v7, o instante de agora e o nome da máquina), grava no banco e aplica na projeção.
5. **`vamo_tui::Voice`:** o dicionário escreve "GOOOL! T-041 ..." (ou "Concluída: T-041 ..." no modo neutro), e o `main` imprime.

O relógio, o nome da máquina e o banco aparecem só nos passos 2 e 4, que são do binário. O passo 3, onde mora a regra, é código puro.
