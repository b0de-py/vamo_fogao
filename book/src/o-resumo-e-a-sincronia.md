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
