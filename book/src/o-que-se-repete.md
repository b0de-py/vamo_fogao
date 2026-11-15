# O que se repete

A fase 6 (Jogando no Niltão) separou três jeitos de uma coisa voltar, cada um no seu lugar. Este capítulo mostra como escolher e por onde o código passa. As decisões estão nas ADRs [0014](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0014-recorrencia-mora-na-agenda.md), [0015](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0015-tarefa-que-volta-por-intervalo.md) e [0016](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0016-jogada-em-sequencia.md). Os exemplos são inventados.

## Qual dos três

| A coisa... | Mora em | Como aparece |
|------------|---------|--------------|
| se repete todo dia ou em certos dias da semana | A agenda, como evento repetido | Fixa na tela Hoje, com tick. O app só lê. |
| volta de tantos em tantos meses ou semanas, sem horário | Uma tarefa com intervalo | Leva gol, some e volta pro Banco. |
| tem passos em ordem, e volta no mês seguinte | Uma tarefa com passos (a jogada) | Só o passo da vez aparece. |

O que saiu foi o status `recorrente`, que só queria dizer "todo dia, pra sempre, sem gol". O nome antigo ainda é lido como `pronta`, pra o banco e os exports antigos carregarem.

## A tarefa que volta

```text
vamo add "Regar as plantas" --volta 2s
vamo volta 7 1m        # troca o intervalo
vamo volta 7 nunca     # deixa de voltar
```

O intervalo (`Interval`) conta a partir do dia do gol, e o mês curto não estoura: 31 de janeiro mais um mês é o fim de fevereiro. Quem calcula é a `Projection`, sem ler o relógio:

- `Task::returns_on` diz o dia em que a tarefa concluída volta.
- `Projection::resurface(hoje)` devolve as que já deviam ter voltado, com as mudanças (pronta, sem dia).
- `Projection::coming_back(hoje)` lista as que ainda dormem, pra faixa "Voltam em breve" do Banco.

Quem grava as mudanças é o binário: o `Session::open` chama `resurface` em todo comando. Como a tarefa deixa de estar concluída, rodar de novo não repete nada.

## A jogada em sequência

```text
vamo add "Fechar o mês" \
  --passo "Pedir as datas + Recebi as datas + Recebi o e-mail" \
  --passo "Montar o rascunho @48h" \
  --passo "Revisar o rascunho @pedir"
```

Pra transformar uma tarefa que já existe, mantendo o ID e o histórico, use `vamo jogada 7 --passo "..." --passo "..."`.

Cada `--passo` é um título, um prazo opcional e itens opcionais:

- `@48h` vence 48 horas depois de fechar o passo anterior (horas contam em dias cheios).
- `@pedir` pergunta a data na hora de fechar o passo anterior (`vamo done 7 --data 2026-12-15`, ou a linha de digitar a data na TUI).
- `+ item` é uma coisa que o passo espera. Os itens se marcam em qualquer ordem (`vamo item 7 2`), e o passo só fecha com todos marcados.

O prazo vira o dia da escalação do próximo passo: ele aparece no Hoje nesse dia e, se passar, no atraso, em amarelo plasma. Se o passo depende de algo de fora, `vamo espera 7` tira a jogada da escalação e a põe na faixa "Aguardando" do Banco, com o que falta. Esperar não é atraso e não quebra a sequência de dias.

No código, a jogada são dois campos da `Task`: o `Plan` (os passos, imutáveis) e o `Progress` (o passo da vez, os itens marcados, a espera). Os dois viajam pelo log como `Change::Plan` e `Change::Progress`, e quem decide o que fechar um passo faz é `Projection::finish_with`. Fechar o último passo é o gol, e a jogada dorme até o primeiro dia do mês seguinte (`Task::returns_on`).
