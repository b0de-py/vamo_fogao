# Glossário

O código fala inglês e a interface fala português, com a linguagem de jogo ligada por padrão. Este glossário liga uma ponta à outra.

## Do domínio ao código

| No app | No código | O que é |
|--------|-----------|---------|
| Tarefa | `Task` | Uma coisa a fazer, com título, área, encaixe e status. |
| ID da tarefa | `TaskId` | O número no formato `T-001`. |
| Área | `Area` | O pedaço da vida a que a tarefa pertence (saúde, casa, estudos...). As áreas vêm da config. |
| Encaixe | `Fit` | Que tipo de espaço a tarefa pede: foco, brecha ou com alguém. |
| Status | `Status` | Em que pé a tarefa está: ideia, destrinchando, pronta, agendada, recorrente, concluída ou descartada. |
| Caderno | notebook | A lista do dia no formato do caderno de papel: título curto, verbo na frente. |
| Título do caderno | `NotebookTitle` | Um título de até cinco palavras que começa com verbo. |
| Brecha | `Gap` | Uma janela livre no expediente, onde cabe uma tarefa curta. |
| Compromisso | `Appointment` | Um evento da agenda, já no horário local, com a recorrência resolvida. |
| Rodapé | `Footer` | A linha `vamo: T-041 \| ...` na descrição do evento, que liga ele à tarefa. |
| Atividade fixa da agenda | `FixedActivity` | Um compromisso recorrente, de marcar na tela Hoje sem virar tarefa. |
| Regras do tempo | `TimeRules` | Expediente, teto da noite e blocos protegidos, da seção `[tempo]` da config. |
| Regra ferida | `Violation` | O que uma tarefa marcada num horário fere: o teto da noite ou um bloco protegido. |
| Caixa a marcar | inbox | As tarefas prontas sem dia, esperando horário na agenda. |
| Gol | done | Tarefa concluída. |
| Evento | `Event` | Uma mudança numa tarefa, com o instante, a máquina e um ID que nunca se repete. |
| Mudança | `Change` | O que um evento mudou: a criação da tarefa ou um campo só. |
| Projeção | `Projection` | O estado das tarefas calculado a partir do log de eventos. |
| Máquina | `Machine` | O nome do PC onde a mudança aconteceu. |
| Banco | `Store` | O SQLite onde o log de eventos fica guardado. |

## Do acervo ao código

| No app | No código | O que é |
|--------|-----------|---------|
| Acervo | `vamo-acervo` | O museu de bolso: os fatos da história do Botafogo. |
| Fato | `Fact` | Um fato do acervo, com texto, data, tags e fontes. |
| Curiosidade do dia | daily fact | O fato verificado que abre o dia. Chega na fase 2. |
| Resenha | `Banter` | A zoeira, absurda de cara e com selo próprio. |
| Fonte | `Source` | De onde o fato veio: o nome e, se tiver, o link. |
| Origem | `Origin` | O domínio de uma fonte, pra régua de independência. |
| Verificado, a conferir | `Verification` | Se o fato já tem as duas fontes independentes. |
| Recorde | `Kind::Record` | Um fato que pode cair, e por isso precisa de data. |

Os nomes do código sem crase ainda vão nascer nas próximas fases; o glossário acompanha.

## A linguagem de jogo

A interface fala a língua da arquibancada. Quem preferir liga o modo neutro na config, e os termos da primeira coluna voltam:

```toml
# ~/.config/vamo_fogao/config.toml
linguagem = "neutra"
```

Pra uma vez só, vale `vamo --linguagem neutra` ou a variável `VAMO_LINGUAGEM`, que ganham da config.

| Modo neutro | Linguagem de jogo | Por quê |
|-------------|-------------------|---------|
| Banco de tarefas | Banco | Tarefa parada é reserva esperando a chance. |
| Tarefas do dia | Escalação | Você escala o dia, e quem sai do banco entra em campo. |
| Concluir | Gol | Um GOL! pisca na linha. |
| Desfazer | VAR | O VAR anulou, e a tarefa volta pro jogo. |
| Adiar | Volta pro banco | Não jogou hoje, sem drama. |
| Feito e em aberto | Placar do dia | 4 x 2 quer dizer quatro feitas e duas em aberto. |
| Foco, brecha, com alguém | Concentração, contra-ataque, tabelinha | Jogo longo, janela curta e lance a dois. |
| Regra do tempo ferida | Impedimento | O app marca; você decide se segue o lance. |
| Vencida | Acréscimos | Passou do prazo, mas ainda dá. |
| Caixa pra agendar | A marcar | Como jogo sem data na tabela. |
| Tarefa que chega de fora | Reforço | Contratar ou dispensar com uma tecla. |
| Resumo do dia | Súmula | O apito final das 4h fecha a súmula. |
| Semana e revisão semanal | Rodada e coletiva | A revisão da semana é a coletiva da rodada. |
| Dias seguidos sem vencida | Invencibilidade | O recorde a bater é o do clube: 52 jogos, em 1977 e 1978. |
