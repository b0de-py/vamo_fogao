# A agenda

A fase 3 traz a agenda pro app. O `vamo` lê os compromissos, acha as brechas entre eles, avisa quando uma tarefa marcada fere uma regra do tempo e liga cada evento à tarefa dele pelo rodapé `vamo:`. Este capítulo mostra como configurar, como usar e por onde o código passa. As decisões estão nas ADRs [0008](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0008-agenda-por-ics-com-leitor-proprio.md) (o leitor de ICS), [0009](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0009-marcacao-da-agenda-fora-do-log.md) (a marcação das fixas) e [0010](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0010-google-agenda-por-oauth-atras-de-feature.md) (o Google Agenda).

## A config

Tudo mora em `~/.config/vamo_fogao/config.toml`. O exemplo abaixo é inventado:

```toml
[agenda]
# O link secreto da agenda em formato iCal. É segredo: fica só aqui.
ics = "https://agenda.exemplo/privado/basic.ics"

[tempo]
expediente = "09:00-18:00"
dias_de_expediente = ["seg", "ter", "qua", "qui", "sex"]
teto_da_noite = "22:00"
brecha_minima = 15

[[tempo.protegido]]
nome = "Almoço"
horario = "12:00-13:00"
dias = ["seg", "ter", "qua", "qui", "sex"]

[apelidos]
"Reunião diária da equipe de produto" = "Daily"
```

- **`[agenda]`:** de onde a agenda vem. Sem a seção, os comandos da agenda dizem como configurar.
- **`[tempo]`:** as regras do tempo. O expediente é onde as brechas são procuradas; sem os dias, vale de segunda a sexta. O teto da noite vale pra tarefa de foco. A brecha mínima é em minutos, e o padrão é 15.
- **`[[tempo.protegido]]`:** horários que não são de tarefa. Sem `dias`, vale todo dia.
- **`[apelidos]`:** o título curto de um evento recorrente de nome comprido, pela chave do título do evento (sem ligar pra maiúscula).

Campo que o `vamo` não conhece é erro, pra um erro de digitação não passar batido.

## Os comandos

| Comando | O que faz |
|---------|-----------|
| `vamo agenda` | Os compromissos do dia (ou de `--data`), com o aviso das regras feridas e as brechas, cada uma com as tarefas que cabem nela. |
| `vamo sync` | Baixa a agenda e liga cada evento com rodapé à tarefa dele, que fica agendada no dia do evento. Duas semanas pra trás e três meses pra frente. |
| `vamo caixa` | A caixa "a marcar": as tarefas prontas sem dia, cada uma com o encaixe, a estimativa e o rodapé pra colar no evento. |
| `vamo conectar-google` | O login no Google Agenda, com a feature `google`. |

Na TUI, a tela Hoje ganha a agenda do dia: as fixas da agenda como itens de marcar, os compromissos com as regras feridas em amarelo plasma e as brechas. A TUI lê só a cópia local da agenda, pra abrir na hora sem esperar a internet; quem atualiza a cópia é o `vamo sync` (e o `vamo agenda`).

## O rodapé `vamo:`

O rodapé é o contrato entre a agenda e o app. É uma linha na descrição do evento:

```text
vamo: T-041 | Trocar o filtro da água | casa | brecha
```

Só o ID é obrigatório. O título, a área e o encaixe estão lá pra quem lê a agenda, e o `vamo` confia no banco. O leitor é tolerante: aceita `t-41` e `T041`, maiúscula e espaço sobrando, e `|` dentro do título. Vale a última linha que começa com `vamo:`.

O caminho de uma tarefa até a agenda é este:

1. A tarefa fica pronta e sem dia: ela aparece no `vamo caixa`.
2. Alguém (você, ou o Claude lendo a caixa) cria o evento na agenda e cola o rodapé na descrição.
3. No próximo `vamo sync`, o evento liga à tarefa, que fica agendada no dia dele e sai da caixa. Se o evento mudar de dia, o sync muda o dia da tarefa junto.

Tarefa concluída, descartada ou recorrente não muda pelo sync: o evento é só lembrança. Ideia e destrinchando também não, com um aviso dizendo que ela precisa ficar pronta antes.

## As brechas

A brecha é o contra-ataque: uma janela livre no expediente. A conta mora em `crates/vamo-core/src/gaps.rs` e é pura: o dia, a hora de agora e os compromissos chegam por parâmetro.

1. O dia começa como o expediente inteiro (hoje, a partir de agora).
2. Os compromissos com horário e os blocos protegidos ocupam pedaços dele.
3. O que sobra, com pelo menos a brecha mínima, é brecha.

Pra cada brecha, `fitting` sugere as tarefas de encaixe brecha que estão prontas, ainda sem horário na agenda e que cabem nela. O tamanho sai da estimativa escrita nas notas: `~30 min`, `~1 h`, `~1h30`. Tarefa sem estimativa cabe em qualquer brecha, porque brecha é coisa rápida por definição.

Os testes de propriedade garantem as três regras que não podem falhar: a brecha nunca cobre um compromisso, nunca sai do expediente e nunca fica menor que o mínimo.

## As regras do tempo

`TimeRules::check` diz que regra uma tarefa fere se for feita num horário. São duas, as duas marcadas em amarelo plasma (é impedimento, não é proibido: o app marca, você decide se segue o lance):

- **Teto da noite:** tarefa de foco que termina depois do teto, ou passa da meia-noite.
- **Bloco protegido:** qualquer tarefa que encosta por dentro num bloco protegido do dia. Encostar na borda não conta.

## As atividades fixas

Os compromissos recorrentes da agenda (a caminhada, a reunião diária) aparecem na Hoje num grupo à parte, "Fixas da agenda", como itens de marcar. O título é o apelido da config ou o título do evento cortado no tamanho do caderno, do mesmo jeito que a sugestão de corte do título de tarefa.

Marcar uma delas conta no placar do dia, mas não vira tarefa: a marcação mora numa tabela própria do banco, `agenda_checks`, com a chave dia e UID do evento (migração 0003). O log de eventos é todo de tarefa, e um compromisso que já mora na agenda viraria duplicata a cada sync.

## As duas portas

O core conhece a agenda só pela porta `Agenda` e pelo tipo `Appointment`: um compromisso já no horário local, com a recorrência resolvida e a tarefa ligada, se tiver. Duas portas implementam isso no `vamo-agenda`:

- **O link ICS** (`IcsLink` e `Calendar`), o padrão. O leitor do ICS e da regra de recorrência é feito aqui, no pedaço que as agendas usam. A recorrência é expandida no fuso do próprio evento e só depois levada pro fuso de quem usa: a reunião das 9h continua às 9h depois da virada do horário de verão. O `BYSETPOS` também entra, pra regras como "o último dia útil entre 28 e 30". Regra com parte que o leitor não conhece (como `BYHOUR`) não inventa ocorrência: o evento fica de fora, com um aviso.
- **O Google Agenda** (`GoogleLink` e `GoogleCalendar`), atrás da feature `google`. A API já devolve a recorrência expandida, e a resposta vira o mesmo `Appointment`. Um teste lê uma resposta gravada da API e a mesma semana escrita em ICS, e as duas têm que dar os mesmos compromissos.

As duas guardam uma cópia local em `~/.cache/vamo_fogao/`, e sem internet vale a cópia, com aviso. No binário, as duas viram um `Source` só (`crates/vamo_fogao/src/agenda.rs`), e o resto do app não sabe de qual veio.

### Ligar o Google Agenda

1. Compile com a feature: `cargo install --path crates/vamo_fogao --features google`.
2. No Google Cloud, crie um projeto, ative a API do Google Calendar e crie um cliente OAuth do tipo "App para computador".
3. Na config, troque a fonte e ponha o cliente:

   ```toml
   [agenda]
   fonte = "google"
   google_cliente = "123-exemplo.apps.googleusercontent.com"
   google_segredo = "segredo-do-cliente"
   # google_agenda = "primary"  (a principal, se ficar de fora)
   ```

4. Rode `vamo conectar-google`. O navegador abre na tela de permissão do Google, só de leitura; ao aceitar, o token fica em `~/.config/vamo_fogao/google-token.json`, só com permissão pra você. Apagar o arquivo desconecta.
