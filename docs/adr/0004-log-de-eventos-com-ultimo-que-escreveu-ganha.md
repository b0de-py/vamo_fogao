# 0004. Log de eventos com "o último que escreveu ganha" por campo

- **Status:** aceita
- **Data:** 2026-11-11

## Contexto

O vamo_fogao vai rodar em mais de uma máquina (o PC e o notebook, por exemplo), cada uma com o próprio banco, e as duas precisam chegar no mesmo estado sem servidor no meio. A sincronia vai ser por arquivo, e um arquivo pode chegar atrasado, chegar duas vezes ou chegar depois de uma mudança feita do outro lado. Além disso, é útil saber quando e onde cada tarefa mudou, pra súmula e pra revisão da semana.

## Decisão

O estado das tarefas não é guardado: é calculado a partir de um log de eventos.

- Toda mudança vira um `Event` com ID UUID v7, instante (`jiff::Timestamp`), máquina de origem, tarefa e o que mudou (`Change`).
- A criação preenche todos os campos; as outras mudanças mexem em um campo só.
- A `Projection` aplica os eventos. Cada campo fica com o valor do evento mais recente que mexeu nele: maior instante e, no empate exato, maior ID.
- A projeção não confere regra. Título do caderno e transição de status são conferidos antes, na hora de criar o evento; depois que ele existe, é fato.
- O core só confere o ID. Quem gera o UUID e lê o relógio é o binário.

## Alternativas consideradas

- **Guardar só o estado atual e sincronizar a tabela inteira:** simples, mas a máquina que sincroniza por último apaga o que a outra fez, inclusive em campos que ela nem tocou.
- **"O último que escreveu ganha" por tarefa inteira:** perde a mudança de uma máquina sempre que a outra mexe em qualquer campo da mesma tarefa.
- **CRDTs mais ricos (registro com vários valores, texto colaborativo):** resolvem casos que um app de uma pessoa só quase não tem, ao custo de mostrar conflito pra ela resolver na mão.
- **Ordenar por relógio lógico (Lamport) em vez do relógio de parede:** aguenta relógio errado, mas o instante deixa de dizer quando a coisa aconteceu, e a súmula precisa disso.

## Consequências

- Aplicar os mesmos eventos em qualquer ordem dá o mesmo estado, e receber um evento duas vezes não muda nada. Os testes de propriedade da projeção garantem isso.
- Sincronizar vira juntar logs: cada máquina manda os eventos que a outra não tem.
- Uma máquina com relógio muito atrasado pode perder uma mudança pra outra mais antiga. O NTP do sistema resolve na prática.
- Mudança que chega antes da criação da tarefa fica guardada esperando; a tarefa só aparece quando a criação chegar.
- O log só cresce. Pra uma pessoa são alguns milhares de eventos por ano, e recalcular tudo na abertura leva milissegundos. Se um dia pesar, dá pra guardar um retrato do estado sem mudar a regra.
- Duas máquinas criando tarefa ao mesmo tempo podem escolher o mesmo número `T-###`. Isso fica pra fase 5, a da sincronia, resolver.
