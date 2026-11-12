# 0009. Guardar a marcação das atividades fixas da agenda fora do log de eventos

- **Status:** aceita
- **Data:** 2026-11-12

## Contexto

Os compromissos recorrentes da agenda (a caminhada, a reunião diária) aparecem na tela Hoje como itens de marcar, e marcar um deles conta no placar do dia. A regra da tarefa é clara: isso não pode virar tarefa nova no banco, porque o compromisso já mora na agenda e a cada sync voltaria em duplicata. Mas o log de eventos (ADR 0004) é todo de tarefa: cada evento tem um `TaskId`, e a projeção só sabe montar tarefa.

## Decisão

- A marcação vai numa tabela própria, `agenda_checks`, com a chave `(dia, UID do evento)`, criada pela migração 0003. Marcar insere a linha; desmarcar apaga.
- O core ganha `FixedActivity` e `fixed_activities`: dos compromissos recorrentes do dia, um item por UID, com o título curto (o apelido da seção `[apelidos]` da config ou o título do evento cortado no tamanho do caderno). O placar soma as marcadas com `Score::plus`.
- A porta `Ledger` da TUI ganha `agenda_checks` e `check_agenda`. O binário carrega a tabela inteira na abertura (é pequena) e grava a cada marcação.

## Alternativas consideradas

- **Um evento no log com um ID de tarefa reservado:** mentiria pro resto do sistema. A projeção criaria uma tarefa fantasma, e a súmula e o `vamo list` teriam que saber esconder.
- **Uma variante de evento sem tarefa no log:** o caminho certo quando a sincronia entre máquinas chegar, mas mexe no tipo `Event`, na projeção, no esquema do log e no formato exportado de uma vez. É grande demais pra uma tarefa desta fase.
- **Guardar na TUI, só em memória:** a marcação sumiria ao fechar o app, e o placar do dia mudaria entre uma abertura e outra.

## Consequências

- A marcação da agenda não viaja com o log de eventos: na fase 4 (sincronia), ela precisa entrar no que é exportado, ou virar evento de verdade, com uma ADR nova.
- O UID é o que o ICS manda. Se a agenda trocar o UID de um evento recorrente (recriar o evento, por exemplo), a marcação antiga não aparece mais, o que é o esperado: é outro compromisso.
