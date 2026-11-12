# A barra e os lembretes

A fase 4 tira o `vamo` do terminal. O primeiro passo é o placar do dia sempre à vista, num módulo da barra do Omarchy: `⚽ 3 x 2` quer dizer três feitas e duas em aberto. Com o mouse em cima, a dica mostra o dia, o placar, o atraso, o próximo compromisso da agenda e o que ainda está em campo. O clique esquerdo mostra e esconde a TUI; o direito relê o placar na hora.

## O comando

```sh
vamo barra          # ⚽ 3 x 2 (no modo neutro, 3/5)
vamo barra --json   # o mesmo, no formato do módulo
```

O placar é o mesmo da tela Hoje, com as fixas do banco e as da agenda marcadas: a conta mora no core (`Lineup::score_with`) e as duas telas só mostram. A agenda vem da cópia local, a que o `vamo sync` baixa. A barra pergunta a cada meio minuto e não pode esperar a internet.

## O formato

O `--json` é um contrato entre o binário e o módulo, e os dois podem estar em versões diferentes: quem atualiza o `vamo` nem sempre copia o módulo de novo. Por isso a saída tem o campo `versao`, e um teste de snapshot (`the_json_format_is_stable`, em `crates/vamo_fogao/src/bar.rs`) quebra se o formato mudar sem querer. O exemplo é inventado:

```json
{
  "versao": 1,
  "dia": "2026-11-12",
  "text": "⚽ 1 x 2",
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
| `class` | Uma de `vazio` (nada previsto), `andando` ou `fechado` (tudo feito), mais `atraso` quando tem tarefa atrasada. |
| `placar` | Os números do placar: `feitas` e `previstas`. |
| `atrasadas` | Quantas tarefas escaladas pra um dia que passou ainda não saíram. |
| `proximo` | O próximo compromisso com horário de hoje (`inicio` e `titulo`), ou `null`. |

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
