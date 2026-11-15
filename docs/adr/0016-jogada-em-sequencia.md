# 0016. Fazer a jogada em sequência com passos, itens e o ponto no log

- **Status:** aceita
- **Data:** 2026-11-15

## Contexto

Há trabalho que anda em ordem e se repete: um ciclo mensal de passos, onde o primeiro depende de duas coisas de fora (que chegam em qualquer ordem), o segundo vence um tempo depois de o primeiro fechar e os seguintes têm data informada na hora. Uma tarefa por passo enche o Banco de coisas que não podem ser feitas ainda; uma tarefa só, com a ordem nas notas, não deixa o app saber qual é a vez. E o passo que espera um e-mail não pode travar o resto, nem quebrar a sequência de dias.

## Decisão

- **Uma tarefa guarda os passos** (`Plan`, de 1 a 12 `Step`) e **o ponto** (`Progress`: o passo da vez, os itens marcados e se espera). São dois eventos novos do log, `Change::Plan` e `Change::Progress`. O ponto vai inteiro em cada evento, e vale o mais recente (o "último que escreveu ganha" da ADR 0004, no campo `progress`).
- **Só o passo da vez aparece**: as telas e a CLI mostram o título da tarefa com o passo da vez, e a ficha mostra todos, com o da vez em destaque.
- **O passo tem prazo** (`@48h`, horas depois de fechar o anterior, contadas em dias cheios; ou `@pedir`, a data que o app pergunta ao fechar o anterior). O prazo vira o dia da escalação do próximo passo: ele aparece na escalação desse dia e, se passar, no atraso, em amarelo plasma. Sem prazo, o passo espera no Banco.
- **Passo com itens** (`+ item`, até 8) só fecha com todos marcados, em qualquer ordem. É o caso de duas coisas de fora que chegam em ordem qualquer.
- **Aguardando**: a jogada pode esperar algo de fora. Ela sai da escalação, fica numa faixa "Aguardando" do Banco com o que falta, e como deixou de estar escalada, não vira atraso nem quebra a sequência de dias.
- **Fechar o último passo é o gol.** A jogada recomeça do primeiro passo e dorme até o primeiro dia do mês seguinte, quando volta pro Banco (a mesma devolução da ADR 0015).
- **Na CLI**: `vamo add ... --passo "Título @48h + item"` (uma vez por passo), `vamo done <ID> [--data D]`, `vamo item <ID> <N>` e `vamo espera <ID>`. Na TUI, o espaço fecha o passo e abre a linha de digitar a data quando o próximo pede, e na ficha `i` marca um item e `w` põe em espera.
- **Formato de exportação 2** (ainda não lançado, então a pasta de schema v2 foi refeita): linhas `jogada` e `ponto`.

## Alternativas consideradas

- **Uma tarefa por passo, ligadas por dependência:** o Banco mostraria o que ainda não dá pra fazer, e o app precisaria de um grafo só pra dizer "essa vem depois".
- **Prazo como data absoluta guardada no passo:** o prazo é relativo ao fechamento do anterior, e só então se sabe a data; por isso ela nasce ao fechar, como dia da escalação.
- **O ponto como vários eventos pequenos (um por item):** resolveria conflito item a item, mas multiplica tipos de evento por um caso de um usuário só; o ponto inteiro basta.

## Consequências

- Duas máquinas que mexem no ponto ao mesmo tempo ficam com a mais recente; um item marcado numa e outro na outra pode perder um dos dois. É o preço do ponto inteiro.
- Trocar os passos de uma jogada em andamento (`Change::Plan`) volta o ponto pro começo.
- A tarefa recorrente do banco de quem usa que for uma jogada precisa ser recriada com os passos; a migração dos dados reais é feita na máquina de cada um, e não no repositório.
