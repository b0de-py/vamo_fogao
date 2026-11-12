# 0011. Mandar os lembretes por um timer do systemd e pelo notify-send

- **Status:** aceita
- **Data:** 2026-11-12

## Contexto

O lembrete precisa chegar na hora certa mesmo com a TUI fechada, nas notificações do desktop (as do Omarchy, que falam o protocolo `org.freedesktop.Notifications` pelo D-Bus). O `vamo` não tem processo que fica de pé: cada comando abre o banco, faz o que tem que fazer e sai. O core não lê o relógio, e o envio precisa ser testável sem desktop.

## Decisão

- Um comando curto, o `vamo lembrar`, faz uma rodada: olha da última rodada até agora, avisa o que venceu e guarda até onde olhou em `$XDG_STATE_HOME/vamo_fogao/lembretes`. Quem chama é um timer do systemd do usuário, de minuto em minuto (`extras/systemd/`).
- A conta é do core: a janela da rodada (`ReminderWindow::since`, que não volta mais de 15 minutos, pra máquina que acorda de manhã não despejar os avisos da noite) e os avisos que vencem nela (`reminders`). O começo da janela fica de fora, então rodadas seguidas avisam cada compromisso uma vez só; um teste de propriedade confere.
- O envio fica atrás da porta `Notifier` do core. O binário implementa com o `notify-send` da libnotify, que fala com o serviço de notificações pelo D-Bus. Nos testes, um falso guarda os avisos.
- A agenda vem da cópia local, como na barra. Quem mantém a cópia em dia é o `vamo sync`, com um segundo timer opcional, a cada meia hora.

## Alternativas consideradas

- **Falar D-Bus direto, com o `zbus`:** é o caminho sem programa de fora, mas traz um runtime assíncrono e algumas dezenas de crates pra mandar uma mensagem por vez. O `notify-send` já vem com o desktop.
- **Um processo que fica de pé (`vamo lembrar --daemon`):** precisa cuidar de dormir e acordar, de recarregar a agenda e de morrer junto com a sessão. O timer do systemd já faz tudo isso, e o `vamo` continua sem estado em memória.
- **Agendar cada aviso no systemd (`systemd-run --on-calendar`):** cada mudança na agenda pediria desmarcar e marcar de novo. A rodada de minuto em minuto sempre lê a agenda do momento.
- **Guardar a última rodada no banco:** o banco é o que vai pra sincronia (fase 5), e a última rodada é de cada máquina.

## Consequências

- O aviso chega com até um minuto de atraso (o `AccuracySec=1s` do timer tira o atraso extra do systemd).
- Sem o `notify-send` instalado, a rodada falha com uma mensagem que diz o que falta, e o `vamo lembrar --teste` serve pra conferir a instalação.
- Um aviso que falha não volta na rodada seguinte: a janela anda mesmo assim, pra não repetir os que saíram.
