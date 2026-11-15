# 0015. Fazer a tarefa que volta por um intervalo contado do gol

- **Status:** aceita
- **Data:** 2026-11-15

## Contexto

Com o fim do status "recorrente" (ADR 0014), sobrou o que se repete sem horário: uma tarefa mensal, ou de tantas em tantas semanas, que não cabe como evento da agenda. Ela precisa levar gol normal (é uma entrega, não um hábito), sumir da vista e voltar pro Banco sozinha, sem ninguém lembrar de olhar.

## Decisão

- **A tarefa ganha um intervalo** (`Interval`: `1m` todo mês, `2s` a cada duas semanas, de 1 a 99), por um evento novo do log, `Change::Repeat`. O intervalo conta **a partir do dia do gol**, e mês curto não estoura: 31 de janeiro mais um mês dá o fim de fevereiro.
- **O dia de voltar é calculado, não guardado**: concluída, com intervalo, o dia do gol mais o intervalo (`Task::returns_on`). Quem dorme aparece na faixa "Voltam em breve" do Banco, com a data (`Projection::coming_back`).
- **Quem devolve é a projeção, e quem grava é o binário**: `Projection::resurface(hoje)` diz quais tarefas já voltaram e as mudanças que as devolvem (pronta, sem dia). O `Session::open` aplica isso em todo comando, inclusive o da barra e o do timer, então a tarefa reaparece no dia certo sem pedido. É evento no log, então viaja entre máquinas, e rodar de novo não repete, porque a tarefa deixou de estar concluída. O core continua sem ler o relógio: o dia chega por parâmetro.
- **Não tem prazo, então não atrasa**: a tarefa que volta espera no Banco.
- **Formato de exportação 2**: o evento novo vira `volta` na linha do log. A versão sobe, a pasta de schema v2 e os exemplos v2 entram, e a leitura da versão 1 continua. A migração 0006 do SQLite refaz a tabela de eventos pra aceitar o tipo novo.
- **Na CLI**: `vamo add ... --volta 1m` e `vamo volta <ID> <1m|2s|nunca>`.

## Alternativas consideradas

- **Guardar o dia de voltar num evento agendado:** exigiria alguém gravar o evento no futuro e dobraria a fonte da verdade; calcular do gol e do intervalo deixa um dado só.
- **Dia fixo do mês:** serve pra conta que vence todo dia 10, mas isso é data da agenda; o pedido foi "a partir do gol". Pode virar opção depois.
- **Devolver só na hora de desenhar a tela, sem gravar:** a tarefa voltaria nas telas, mas continuaria concluída no log, na súmula e no resumo, e cada tela teria que repetir a regra.

## Consequências

- A tarefa que voltou perde o dia do gol (volta sem dia, pronta), e o gol anterior fica no histórico e na súmula do dia em que saiu.
- Duas máquinas podem devolver a mesma tarefa ao mesmo tempo: sai um par de eventos repetido, com o mesmo efeito.
- Quem tem o `vamo` velho recusa a linha `versao: 2` do backup, com a mensagem que pede pra atualizar (ADR 0012).
