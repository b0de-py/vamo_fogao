# A barra, os lembretes e o dia de jogo

A fase 4 tira o `vamo` do terminal. O primeiro passo é o placar do dia sempre à vista, num módulo da barra do Omarchy: `⭐ 3 x 2` quer dizer três feitas e duas em aberto, com a Estrela Solitária na frente. Com o mouse em cima, a dica mostra o dia, o placar, o atraso, o próximo compromisso da agenda e o que ainda está em campo. O clique esquerdo mostra e esconde a TUI; o direito relê o placar na hora.

## O comando

```sh
vamo barra          # ⭐ 3 x 2 (no modo neutro, 3/5)
vamo barra --json   # o mesmo, no formato do módulo
```

O placar é o mesmo da tela Hoje, com as fixas do banco e as da agenda marcadas: a conta mora no core (`Lineup::score_with`) e as duas telas só mostram. A agenda vem da cópia local, a que o `vamo sync` baixa. A barra pergunta a cada meio minuto e não pode esperar a internet.

## O formato

O `--json` é um contrato entre o binário e o módulo, e os dois podem estar em versões diferentes: quem atualiza o `vamo` nem sempre copia o módulo de novo. Por isso a saída tem o campo `versao`, e um teste de snapshot (`the_json_format_is_stable`, em `crates/vamo_fogao/src/bar.rs`) quebra se o formato mudar sem querer. O exemplo é inventado:

```json
{
  "versao": 1,
  "dia": "2026-11-12",
  "text": "⭐ 1 x 2",
  "tooltip": "Hoje · quinta, 12/11\nPlacar: 1 de 3\n1 atrasada\n...",
  "class": ["andando", "atraso"],
  "placar": { "feitas": 1, "previstas": 3 },
  "atrasadas": 1,
  "proximo": { "inicio": "12:00", "titulo": "Almoço de família" }
}
```

| Campo | O que é |
|-------|---------|
| `versao` | A versão do formato. Campo novo não muda a versão (o módulo ignora o que não conhece); campo que some, troca de tipo ou de sentido muda. |
| `dia` | O dia do placar, `AAAA-MM-DD`. |
| `text`, `tooltip`, `class` | O texto da barra, a dica e as classes, no padrão dos módulos de comando (o mesmo do Waybar). Até um módulo genérico mostra o placar com eles. |
| `class` | Uma de `vazio` (nada previsto), `andando` ou `fechado` (tudo feito), mais `atraso` quando tem tarefa atrasada e `jogo` em dia de jogo. |
| `placar` | Os números do placar: `feitas` e `previstas`. |
| `atrasadas` | Quantas tarefas escaladas pra um dia que passou ainda não saíram. |
| `proximo` | O próximo compromisso com horário de hoje (`inicio` e `titulo`), ou `null`. |
| `jogo` | O jogo do dia, ou `null`. Os campos estão em [O dia de jogo](#o-dia-de-jogo). |

O módulo confere a `versao`. Se o binário falar uma versão que ele não conhece, a barra mostra `vamo ?` e a dica pede pra atualizar o módulo, em vez de mostrar coisa errada.

## Instalar o módulo

O módulo é o [`extras/omarchy/vamo_fogao.qml`](https://github.com/b0de-py/vamo_fogao/blob/main/extras/omarchy/vamo_fogao.qml), no mesmo molde dos módulos QML da barra do Omarchy.

1. Instale o `vamo` no PATH: `cargo install --path crates/vamo_fogao`.
2. Copie o módulo pra pasta dos seus módulos:

   ```sh
   mkdir -p ~/.config/omarchy/bar/modules
   cp extras/omarchy/vamo_fogao.qml ~/.config/omarchy/bar/modules/
   ```

3. No `~/.config/omarchy/shell.json`, ponha a entrada num dos lados da barra (`left`, `center` ou `right`):

   ```json
   {
     "id": "vamo_fogao",
     "type": "qml",
     "source": "~/.config/omarchy/bar/modules/vamo_fogao.qml"
   }
   ```

4. Recarregue a barra com `omarchy restart shell`.

A entrada aceita, tudo opcional:

| Campo | Padrão | O que faz |
|-------|--------|-----------|
| `vamo` | `vamo` | O comando do `vamo`, se ele não estiver no PATH. |
| `interval` | `30` | De quantos em quantos segundos o placar é relido. |
| `onClick` | o workspace `special:vamo` | O que o clique esquerdo roda. O padrão é o atalho do [capítulo da TUI](a-tui.md#no-hyprland-a-janela-que-aparece-e-some), na sintaxe do Hyprland com config em Lua. Na config clássica, use `hyprctl dispatch togglespecialworkspace vamo`. |
| `colors` | as do r4ul.com.br | A cor de cada classe: `vazio`, `andando`, `fechado` e `atraso`. |

O clique abre a TUI só se o workspace especial estiver configurado como no capítulo da TUI. Sem ele, troque o `onClick` por algo como `alacritty -e vamo tui`.

Se a barra mostrar `vamo !`, o comando deu erro: a dica traz a mensagem. O mais comum é o `vamo` fora do PATH da barra.

## Os lembretes

O segundo passo é o aviso na hora certa, nas notificações do desktop, com a TUI aberta ou não. Dez minutos antes de cada compromisso com horário da agenda, chega uma notificação com o título e a hora: `A bola rola às 15:00` (no modo neutro, `Começa às 15:00`). Se o evento está ligado a uma tarefa pelo rodapé `vamo:`, o título é o da tarefa, com o ID; tarefa concluída ou descartada não tem aviso. A decisão está na [ADR 0011](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0011-lembretes-por-timer-e-notify-send.md).

```sh
vamo lembrar          # uma rodada: avisa o que venceu desde a última
vamo lembrar --teste  # manda uma notificação de teste
```

A antecedência fica na config, em minutos:

```toml
[lembretes]
antecedencia = 15
```

### Como funciona

O `vamo` não fica de pé esperando a hora: um timer do systemd roda o `vamo lembrar` no começo de cada minuto, e cada rodada pergunta ao core "o que venceu entre a última vez que eu olhei e agora?". A última vez fica em `~/.local/state/vamo_fogao/lembretes`.

- **A janela** (`ReminderWindow::since`, em `crates/vamo-core/src/reminder.rs`) vai da última rodada, sem incluir, até agora. Sem incluir o começo, o aviso que caiu bem no fim da rodada anterior não sai de novo. Um teste de propriedade roda um dia inteiro de rodadas e confere que cada compromisso é avisado uma vez só, com qualquer hora e antecedência.
- **A máquina que dormiu** não despeja os avisos da noite de manhã: a janela nunca volta mais de 15 minutos.
- **O envio** passa pela porta `Notifier` do core. O binário manda pelo `notify-send`, que fala com o serviço de notificações do desktop pelo D-Bus; nos testes, um falso guarda os avisos.

A agenda vem da cópia local, a mesma da barra. Quem a mantém em dia é o `vamo sync`, então vale ligar o segundo timer, que sincroniza a cada meia hora.

### Instalar os timers

Os arquivos estão em [`extras/systemd/`](https://github.com/b0de-py/vamo_fogao/tree/main/extras/systemd). Eles chamam o `vamo` em `~/.cargo/bin`, que é onde o `cargo install` põe; se o seu estiver em outro lugar, troque o `ExecStart`.

```sh
vamo lembrar --teste
mkdir -p ~/.config/systemd/user
cp extras/systemd/vamo-* ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now vamo-lembrar.timer vamo-sync.timer
```

Pra conferir: `systemctl --user list-timers 'vamo-*'` mostra a próxima rodada, e `journalctl --user -u vamo-lembrar` mostra os avisos mandados e os erros.

## Os jogos

O terceiro passo é saber quando o Botafogo joga sem depender da agenda pessoal. Quem usa assina a tabela de jogos do time, como se assina uma agenda, e põe o link na config:

```toml
[jogos]
# O link ICS da tabela de jogos (ou o caminho de um arquivo .ics).
ics = "https://tabela.exemplo/jogos.ics"
# O nome do time nos títulos dos jogos. Sem ele, Botafogo.
time = "Botafogo"
```

```sh
vamo jogos                    # os jogos dos próximos 60 dias
vamo jogos --data 2026-11-01  # a partir de outro dia
```

O `vamo sync` baixa a tabela junto com a agenda, e o resto do app lê a cópia local (`~/.cache/vamo_fogao/jogos.ics`). Se a tabela não baixar, o sync avisa e segue com a agenda.

### A porta

O core não sabe de onde vêm os jogos: ele conhece a porta `Fixtures` e o tipo `Game` (o adversário, a hora, o campeonato e se o time é mandante ou visitante), em `crates/vamo-core/src/game.rs`. A primeira implementação é o `IcsFixtures`, do `vamo-agenda`, que lê a agenda ICS com o mesmo leitor da agenda pessoal e reconhece o jogo pelo título do evento. Outra fonte, como uma API de futebol, só precisa devolver os mesmos `Game`s.

As tabelas escrevem o jogo cada uma de um jeito, e o leitor aceita as formas comuns:

| Título do evento | Adversário | Campeonato | Lado |
|------------------|------------|------------|------|
| `Botafogo x Bangu` | Bangu | | mandante |
| `Bangu vs Botafogo` | Bangu | | visitante |
| `⚽ Botafogo x Bangu - Carioca` | Bangu | Carioca | mandante |
| `Carioca: Bangu x Botafogo` | Bangu | Carioca | visitante |
| `Botafogo x Bangu (Copa do Brasil)` | Bangu | Copa do Brasil | mandante |
| `Botafogo-RJ x São Paulo` | São Paulo | | mandante |

Evento que não é jogo do time (um sorteio, um jogo de outros dois times) fica de fora, e jogo sem hora marcada também, porque ainda não tem quando a bola rola. Os testes leem uma tabela de exemplo inventada (`crates/vamo-agenda/tests/fixtures/jogos.ics`), e um teste de propriedade confere que qualquer nome de adversário volta inteiro, dos dois lados do jogo.

## O dia de jogo

Com a tabela de jogos configurada, o dia de jogo tem cara própria. No alto da tela Hoje aparece o card do jogo: o adversário, a hora e o campeonato, as tarefas do jogo e a regra do dia. Na barra, o texto ganha a hora do jogo (`⭐ 3 x 2 · jogo às 21:30`), a dica ganha o jogo e o JSON ganha o campo `jogo`, com a classe `jogo`.

```text
⚽ Dia de jogo
Bangu x Botafogo · 21:30 · Amistoso
 [ ] T-051  Combinar onde ver
 [x] T-052  Separar a camisa
De 19:30 a 23:30, só tarefa leve.
```

- **As tarefas do jogo** são as da área `jogo` (ou a da config, `area = "futebol"` na seção `[jogos]`): como ver, com quem, o que preparar. Entram as que estão em campo, escaladas pro dia ou sem dia, e as que saíram no dia.
- **A regra do dia de jogo:** perto do horário, só tarefa leve. De duas horas antes da bola rolar até o fim do jogo (duas horas depois), tarefa de foco ou com alguém marcada na agenda vira impedimento, em amarelo plasma: `⚠ T-041: pesada perto do jogo (19:30–23:30)`. Brecha pode, e tarefa do próprio jogo também. Nas brechas desse trecho, a tela Hoje só sugere tarefa leve. O `vamo agenda` e o `vamo sync` avisam a mesma regra.

A regra mora no core, no `GameDay` (`crates/vamo-core/src/game.rs`): `light_only` diz o trecho, `check` diz se uma tarefa marcada fere a regra, e `tasks` diz quais são as tarefas do jogo. A TUI, a barra e a CLI só mostram.

No modo neutro, o dia de jogo some de tudo: o `vamo` não fala de futebol.

O campo `jogo` do JSON da barra:

| Campo | O que é |
|-------|---------|
| `adversario` | O adversário, como está na tabela. |
| `inicio` | A hora do jogo, `HH:MM`. |
| `campeonato` | O campeonato, ou `null`. |
| `mando` | `casa` ou `fora`. |
| `so_tarefa_leve` | O trecho perto do jogo: `de` e `ate`, `HH:MM`. |
| `tarefas_em_aberto` | Quantas tarefas do jogo ainda não saíram. |

Sem jogo no dia (ou no modo neutro), `jogo` é `null`. O campo entrou depois da versão 1 do formato e não muda a versão: módulo velho só ignora.

### O fato do confronto

Em dia de jogo, o quadro da curiosidade na tela Hoje mostra a história contra o adversário do dia, quando o acervo tem um fato verificado com a tag `adversario:` dele. Sem fato do confronto, fica a curiosidade de sempre. O detalhe está no capítulo [O acervo](o-acervo.md#o-fato-do-confronto).
