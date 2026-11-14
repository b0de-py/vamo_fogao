# O resumo e a sincronia

A fase 5 fecha o dia sozinha e junta os PCs. O dia termina na súmula das 4h, a semana na rodada, e o log de eventos passa a viajar entre as máquinas por um repositório git privado.

## A súmula do dia

```sh
vamo resumo                    # a súmula do último dia que fechou
vamo resumo --data 2026-11-11  # a de um dia escolhido
vamo resumo --json             # no formato de exportação
```

O dia vai da meia-noite às 4h do dia seguinte. O que se faz de madrugada ainda é da noite anterior, e o apito final das 4h fecha a súmula. Por isso, sem `--data`, o `vamo resumo` mostra o último dia que fechou: às 3h59 ainda é anteontem, às 4h já é ontem.

A regra mora no core, em `DaySummary::close` (`crates/vamo-core/src/summary.rs`). Ela recebe o log inteiro e o trecho do relógio que conta como o dia (`DayWindow`, calculado no binário, que é quem conhece o fuso) e olha o estado das tarefas em dois instantes: no começo do dia e no apito final. Uma tarefa é do dia quando estava escalada pra ele no começo ou foi escalada pra ele durante o dia. No fim, cada uma cai num lugar só:

| Onde | Quando |
|------|--------|
| Gols | Concluída, com o dia da súmula. O `finish` passa a tarefa pro dia do gol, então gol desse dia é gol da súmula, escalada ou não. |
| Fixas marcadas | Recorrente marcada no dia. |
| Voltaram pro banco | Estava no dia e saiu dele sem gol: adiada ou tirada da escalação. |
| Acréscimos | Ficou no dia sem gol. É a tarefa que venceu. |

Descartada não entra em lugar nenhum. Evento depois do apito final não conta, então a súmula de um dia antigo sai igual hoje e daqui a um mês. E a súmula só lê: não conclui, não adia, não mexe em tarefa nenhuma.

O dia que já fechou fica guardado no banco (tabela `summaries`, migração 0004), uma linha por dia: rodar de novo só troca a guardada. É essa cópia que vai pro backup e pro MCP.

### Os hábitos

Os números dos hábitos vêm do habit-tracker, outro app, que guarda cada toque numa tabela SQLite. O core declara a porta `Habits`, e o `vamo-store` implementa com o `HabitTracker`, que abre o banco dele só pra leitura. Na config:

```toml
[habitos]
banco = "~/.local/share/habit-tracker/habits.db"
config = "~/habit-tracker/config.json"
```

Da config do habit-tracker saem o nome e a unidade de cada hábito e a hora em que o dia dele vira (`hora_virada_dia`). Sem a seção, a súmula sai sem hábitos; com o banco quebrado, sai com um aviso e sem eles, porque a súmula das 4h não pode deixar de sair por causa de outro app.

### O timer das 4h

Os arquivos estão em [`extras/systemd/`](https://github.com/b0de-py/vamo_fogao/tree/main/extras/systemd), junto dos outros. O `Persistent=true` faz o timer rodar assim que o PC ligar, se ele estava desligado às 4h.

```sh
cp extras/systemd/vamo-resumo.* ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now vamo-resumo.timer
```

## A invencibilidade

Dias seguidos sem tarefa vencida, com o recorde guardado. O recorde a bater é o do clube: 52 jogos invicto, em 1977 e 1978 (`CLUB_RECORD`, no core).

A conta é a da súmula, dia a dia (`Unbeaten::count`, em `crates/vamo-core/src/unbeaten.rs`). Cada dia fechado vira um `Outcome`:

| Resultado | Quando | A sequência |
|-----------|--------|-------------|
| Vitória (`Win`) | Teve gol e nenhuma tarefa venceu. | Anda um. |
| Derrota (`Loss`) | Uma tarefa escalada ficou no dia sem gol: passou do prazo sem ser feita nem remarcada. | Volta pro zero. |
| Sem jogo (`Idle`) | Nada escalado, ou tudo remarcado, e nenhum gol. | Não anda nem quebra. |

Duas escolhas merecem nota. As fixas ficam de fora: um dia sem marcar a água não quebra a sequência. E remarcar não é perder: tirar a tarefa do dia é decidir, não deixar vencer. A tarefa vencida quebra só o dia dela; no dia seguinte, com gol, a sequência recomeça, mesmo com o atraso ainda na tela.

Como a sequência sai do log, ela não fica guardada em lugar nenhum: o binário conta do dia do primeiro evento até o último dia fechado, e a tela Hoje mostra embaixo do placar (`Invicto há 4 dias · recorde 12`). São alguns milissegundos mesmo com anos de log.

## A rodada da semana

```sh
vamo resumo --semana                    # a semana do último dia fechado
vamo resumo --semana --data 2026-11-11  # a semana desse dia
vamo resumo --semana --json             # o mesmo, pro MCP
```

A semana vai de segunda a domingo, como na ISO 8601 (`WeekSummary::monday_of`). A rodada (`WeekSummary`, em `crates/vamo-core/src/week.rs`) junta a súmula de cada dia, numa linha por dia, os impedimentos, os hábitos somados e a invencibilidade no fim da semana, com o recorde do clube do lado. É o material da coletiva: a revisão semanal curta.

Os impedimentos são as tarefas marcadas na agenda num horário que fere uma regra do tempo (`Offside`): a mesma conta do `vamo agenda`, feita na cópia local da agenda da semana inteira. A rodada não fica guardada; sai sempre do log, e uma semana que ainda não acabou avisa no fim.

## O formato de exportação

Tudo que sai do app tem um formato só, versionado, no módulo `format` do `vamo-sync`. A decisão está na [ADR 0012](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0012-formato-de-exportacao-versionado.md).

| Tipo | Onde aparece | Schema |
|------|--------------|--------|
| `EventLine` | Uma linha do log de eventos exportado (JSONL). | `schema/v1/evento.json` |
| `MarkLine` | Uma marcação de atividade fixa da agenda (JSONL). | `schema/v1/marcacao.json` |
| `SummaryDoc` | A súmula guardada no banco e o `vamo resumo --json`. | `schema/v1/resumo.json` |
| `WeekDoc` | O `vamo resumo --semana --json`. | `schema/v1/rodada.json` |

Uma linha do log, com dados inventados:

```json
{"versao":1,"id":"0192f0c4-8a00-7000-8000-000000000001","em":"2026-11-10T12:00:00Z","maquina":"notebook","tarefa":"T-001","mudanca":{"tipo":"criada","titulo":"Trocar o filtro da água","area":"casa","encaixe":"brecha","status":"pronta"}}
```

Aqui o `serde` aparece com derive pela primeira vez no projeto. Nos outros formatos (a config, o acervo, a API do Google) o código lê campo a campo, pra apontar o erro em português; num contrato entre versões do app, o que pesa é o leitor e o escritor serem o mesmo tipo. O `#[serde(tag = "tipo")]` no enum `ChangeLine` escreve cada variante como um objeto com o nome dela no campo `tipo`, e o `#[serde(deny_unknown_fields)]` faz campo desconhecido virar erro, não silêncio.

**Versão nova lê export antigo.** Cada linha tem a `versao`; a maior que a conhecida é recusada com um pedido de atualizar o `vamo`. O schema de cada tipo sai dos mesmos tipos pelo `schemars` e fica guardado em `crates/vamo-sync/schema/v1/`: o teste `the_schema_of_each_type_matches_the_saved_one` quebra se um tipo mudar sem a `VERSION` subir. Os exemplos de cada versão, em `crates/vamo-sync/tests/fixtures/`, continuam sendo lidos a cada versão nova.
