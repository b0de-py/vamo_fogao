# 0013. Sincronizar pelo git com um arquivo por máquina, renumerando a tarefa não exportada

- **Status:** aceita
- **Data:** 2026-11-14

## Contexto

O log de eventos (ADR 0004) foi feito pra juntar máquinas sem servidor, e a fase 5 é a hora de juntar. Ficaram duas pendências pra esta fase: duas máquinas sem sincronizar podem criar tarefas diferentes com o mesmo `T-###` (ADR 0004), e a marcação das atividades fixas da agenda mora fora do log, numa tabela que não viaja (ADR 0009). O repositório do vamo_fogao é público, e o backup tem tarefa real, então o backup precisa ser outro repositório, privado.

## Decisão

- **O transporte é um clone git**, configurado em `[sincronia] repositorio = "..."`. O `vamo sync` traz (`fetch` e `rebase`), junta, escreve e manda (`commit` e `push`), pelo programa `git` do sistema e com as credenciais que ele já tem. Remoto que termina em `vamo_fogao` é recusado, pra o backup nunca ir pro repositório público.
- **Cada máquina escreve só os arquivos dela**: `eventos/<máquina>.jsonl`, com todos os eventos de origem nela, e `marcacoes/<máquina>.jsonl`, com as marcações que ela conhece. Duas máquinas nunca mexem no mesmo arquivo, então o rebase nunca esbarra em conflito, e juntar é ler os arquivos das outras e guardar (o `append` já ignora evento repetido). Os arquivos são reescritos inteiros em ordem fixa, pra só mudarem quando o conteúdo muda.
- **Se o remoto recusar o envio** porque outra máquina mandou no meio, a rodada se repete do começo, até três vezes.
- **No `T-###` repetido, troca de número quem ainda não exportou** (`renumber_collisions`, no core). Exportado é o evento desta máquina que já está no arquivo dela no backup. Uma tarefa criada aqui, ainda não exportada, com o número de uma que outra máquina criou, passa com todos os eventos não exportados dela pro próximo número livre nos dois lados. É a única vez que um evento guardado muda (`Store::renumber`), e só evento que nenhuma outra máquina viu. O `vamo sync` avisa a troca.
- **A marcação da agenda vira estado com instante**: a tabela ganha `done` e `at` (migração 0005), desmarcar passa a gravar `done = 0` em vez de apagar, e na sincronia a mais recente de cada `(dia, uid)` ganha (`AgendaMark::wins_over`). É o "último que escreveu ganha" do log, num campo só, sem virar evento de tarefa.

## Alternativas consideradas

- **Um arquivo só pro log todo:** toda sincronia de duas máquinas viraria conflito de merge no mesmo arquivo.
- **Trocar o `T-###` por um ID que não colide (UUID, ou o nome da máquina no número):** resolve a colisão, mas o número curto é o que se digita (`vamo done 41`) e o que vai no rodapé da agenda.
- **Renumerar sempre a tarefa mais nova:** a mais nova pode já estar no backup, e outra máquina pode ter escalado, concluído ou posto no rodapé da agenda; trocar o número de algo exportado faz a tarefa sumir do outro lado.
- **A marcação da agenda como evento do log:** exigiria uma tarefa fantasma pra cada atividade fixa, o motivo da ADR 0009 continua valendo.
- **Sincronizar por um serviço (Syncthing, um servidor próprio):** mais uma peça pra instalar e manter; o git já está em todas as máquinas e dá histórico de graça.

## Consequências

- A ADR 0004 e a 0009 têm as pendências de sincronia resolvidas aqui.
- Uma tarefa renumerada que já estava no rodapé de um evento da agenda (`vamo: T-005`) perde a ligação até o rodapé ser trocado; o aviso do `vamo sync` diz qual trocar.
- Se as duas máquinas criarem e mandarem ao mesmo tempo, a segunda tem o envio recusado, traz de novo, vê a colisão e renumera: o caso mais raro cai na mesma regra.
- O `vamo sync` passa a pedir rede pro backup; sem rede, ele falha com o erro do git e a agenda não é lida naquela rodada.
- As marcações antigas, de antes da migração, ficam com o instante zero e perdem de qualquer mudança feita depois em outra máquina.
