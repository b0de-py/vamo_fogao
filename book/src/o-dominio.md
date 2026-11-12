# O domínio

O `vamo-core` é o centro do vamo_fogao: as tarefas e as regras delas, sem banco, sem tela e sem relógio. Este capítulo segue os tipos na ordem em que vale ler os arquivos, todos em `crates/vamo-core/src/`.

## Valor inválido não chega a existir

Todo tipo do domínio confere o valor na construção. Um `TaskId` só existe se o número for válido, um `NotebookTitle` só existe se começar com verbo, e assim por diante. Quem recebe um desses tipos confia nele, sem conferir de novo.

O jeito de fazer isso em Rust é o *newtype*: uma struct com um campo só, privado, e um construtor que confere antes de montar.

```rust,ignore
// area.rs
pub struct Area(String); // o campo é privado: de fora, ninguém monta uma Area na mão

impl FromStr for Area {
    type Err = Error;

    fn from_str(name: &str) -> Result<Self, Error> {
        // confere o nome e só então devolve Ok(Area(...))
    }
}
```

Quase todo tipo do core implementa o par `FromStr` e `Display`: um lê do texto, o outro escreve de volta, e os testes conferem que a ida e a volta dão o mesmo valor. É por isso que o mesmo `concluída` aparece na tela, no banco e na linha de comando.

## Os tipos, um por arquivo

| Arquivo | Tipo | O que garante |
|---------|------|---------------|
| `id.rs` | `TaskId` | O número da tarefa, mostrado como `T-041`. Começa em 1. |
| `area.rs` | `Area` | Um nome bom de digitar: minúsculas (com acento), número e hífen, até 32 caracteres. Quais áreas existem é escolha de quem usa. |
| `fit.rs` | `Fit` | O encaixe: foco, brecha ou com alguém. |
| `status.rs` | `Status` | Em que pé a tarefa está, e pra onde ela pode ir. |
| `title.rs` | `NotebookTitle` | O título no padrão do caderno: até cinco palavras, verbo no infinitivo na frente. |
| `task.rs` | `Task` | A tarefa inteira, montada com as partes acima, com o dia da escalação e as marcações das recorrentes. |
| `lineup.rs` | `Lineup`, `Score` | A escalação de um dia (as escaladas, o atraso e as fixas) e o placar. |
| `reminder.rs` | `ReminderWindow`, `Reminder` | A janela de uma rodada dos lembretes e os avisos que vencem nela. |
| `game.rs` | `Game`, `GameDay` | O jogo do time e o dia de jogo, com as tarefas do jogo e a regra de só tarefa leve perto do horário. |
| `error.rs` | `Error` | Tudo que pode dar errado no core, com a mensagem em português. |

O `Task::new` não devolve `Result`: como cada parte já chega validada, não sobra nada pra dar errado. A validação fica na borda, quando o texto vira tipo, e o meio do código trabalha só com valores bons.

## O status e as transições

`Status::can_become` guarda a regra de quem pode virar o quê, e a tabela inteira cabe num `matches!`:

```rust,ignore
matches!(
    (self, next),
    (Idea, Refining | Ready | Recurring | Dropped)
        | (Refining, Idea | Ready | Recurring | Dropped)
        | (Ready, Refining | Scheduled | Recurring | Done | Dropped)
        | (Scheduled, Ready | Done | Dropped)
        | (Recurring, Ready | Dropped)
        | (Done, Ready)
        | (Dropped, Idea)
)
```

Cada linha é um status de partida e os destinos permitidos. Dois casos contam a história do app: `Done` só volta pra `Ready` (o VAR anulou o gol) e `Dropped` só volta pra `Idea` (a tarefa sai do arquivo e recomeça). Ficar no mesmo status não é transição, então nunca passa.

`Status::transition_to` usa a mesma regra e devolve `Result`, com o erro `InvalidTransition` dizendo de onde pra onde não deu. É ele que a CLI usa no `vamo done`.

## O dia e o placar

A tela Hoje pergunta ao core o que é do dia, e o `Lineup::new(projeção, hoje)` responde em três grupos: as tarefas escaladas pra hoje, as escaladas pra um dia que passou e não foram feitas (o atraso, que a TUI pinta de amarelo plasma) e as recorrentes, as fixas que se marcam todo dia. O placar conta os gols e as fixas marcadas contra tudo que estava previsto pro dia.

Três regras moram na `Projection`, porque são regra de negócio:

- `finish`: o gol conta no dia em que saiu. Concluir uma tarefa atrasada (ou sem dia) passa ela pra hoje.
- `schedule` e `unschedule`: escalar deixa a tarefa agendada, pro dia pedido; tirar da escalação devolve pro banco, pronta.
- `check`: só recorrente se marca, e a marcação é de um dia só. Amanhã a fixa começa desmarcada.

Como o core não lê relógio, "hoje" sempre chega por parâmetro.

## O título do caderno

O caderno de papel tem uma regra: linha curta, verbo na frente. `NotebookTitle::new` confere as duas coisas.

- **Até cinco palavras** (`NotebookTitle::MAX_WORDS`). Título mais longo volta com uma sugestão de corte no erro, e a sugestão já é um título válido: o corte nunca termina em palavra fraca, como "de" ou "com".
- **Verbo no infinitivo na frente.** Não tem dicionário: a regra olha a terminação (`-ar`, `-er`, `-ir`) e a família do "pôr" (`compor`, `repor`...). Erra pouco nos títulos de tarefa e não precisa de lista de verbos.

Como é um parser de texto livre, ele tem teste de propriedade: o proptest gera milhares de títulos e confere, por exemplo, que a sugestão de corte sempre passa na própria regra.

## O que o core não faz

O core não lê arquivo, relógio, variável de ambiente nem banco. Quando uma regra precisa da hora, a hora chega por parâmetro; quando precisa guardar alguma coisa, o core declara uma porta (uma trait) e alguém de fora implementa. É o assunto do [próximo capítulo](o-log-de-eventos.md).
