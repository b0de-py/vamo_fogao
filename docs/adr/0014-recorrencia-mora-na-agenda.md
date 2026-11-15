# 0014. Deixar a recorrência na agenda e tirar o status recorrente

- **Status:** aceita
- **Data:** 2026-11-15

## Contexto

O status "recorrente" significava "todo dia, pra sempre, sem gol", e a tarefa recorrente tinha marcação por dia (`Change::Check`). Na prática confundia: não dizia a frequência, não tinha como ser mensal, e a agenda já resolve o que se repete todo dia ou em certos dias da semana, porque o app lê os eventos repetidos do ICS e já os mostra como fixas da agenda, com tick (ADR 0009). Ter dois jeitos de fixa duplicava a fonte. A ideia do app é gastar o mínimo de token: criar o evento repetido é um custo único, e depois ninguém precisa pedir nada ao Claude.

## Decisão

- **O que se repete por dia ou dias da semana mora na agenda**, como evento repetido criado uma vez. O app só lê, como já fazia. Ele não escreve na agenda.
- **O status `recorrente` sai** do `Status`. Na leitura, o nome antigo continua valendo e vira `pronta` (`FromStr`), então log, banco e exports antigos carregam sem erro, e nenhum evento guardado é reescrito.
- **A marcação por tarefa sai.** `Change::Check` continua existindo no log e nos arquivos de exportação (o formato v1 não muda: `fixas` segue lá, sempre vazia), mas a projeção o ignora. `Projection::check`, `Error::NotRecurring` e o grupo de fixas do `Lineup` saem.
- **O que repete por intervalo** (mensal, a cada N semanas) vira tarefa que volta, e **o que repete em ordem** vira jogada em sequência: duas tarefas desta mesma fase.

## Alternativas consideradas

- **O app criar o evento na agenda:** exigiria escrever na agenda, que o app só lê (ADR 0008), e gastaria token do Claude.
- **O app guardar o hábito localmente:** duplicaria o que a agenda já faz.
- **Reescrever os eventos antigos `recorrente` pra `pronta` no banco:** mexeria em histórico que várias máquinas podem ter; ler o nome antigo como `pronta` resolve sem encostar no log.

## Consequências

- As tarefas que estavam como recorrentes viram prontas. Quem tinha tarefa diária precisa criar o evento repetido na agenda; as outras viram jogada em sequência ou tarefa que volta, conforme o caso.
- Os placares e as súmulas deixam de contar tarefa recorrente marcada; só contam gols e fixas da agenda.
- Um export antigo com `fixas` preenchidas ainda lê; o campo é ignorado.
