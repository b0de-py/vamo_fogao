# 0008. Ler a agenda pelo link ICS, com leitor próprio e cópia local

- **Status:** aceita
- **Data:** 2026-11-12

## Contexto

A fase 3 traz a agenda pro app: os compromissos do dia, as brechas entre eles e o rodapé `vamo:` que liga um evento a uma tarefa. Quase toda agenda (Google, Outlook, Apple, Nextcloud) oferece um link secreto em iCalendar (ICS) que devolve tudo sem login. O ICS tem dois pedaços difíceis: a regra de recorrência (`RRULE`, com exceções em `EXDATE` e `RECURRENCE-ID`) e o fuso de cada horário, inclusive a virada do horário de verão. O check-in da manhã precisa funcionar sem internet.

## Decisão

- O core ganha o tipo `Appointment` (o compromisso já no horário local, com a recorrência resolvida e a tarefa ligada, se tiver) e a porta `Agenda`. Ele não sabe que existe ICS.
- O `vamo-agenda` lê o ICS com um leitor próprio, no pedaço que as agendas usam: `VEVENT`, linhas dobradas, texto com escape, `DTSTART`/`DTEND`/`DURATION`, `TZID` com nome da base IANA, `RRULE` diária, semanal, mensal e anual (com `INTERVAL`, `COUNT`, `UNTIL`, `BYDAY` com posição, `BYMONTHDAY` e `BYMONTH`), `EXDATE`, `RECURRENCE-ID` e `STATUS:CANCELLED`. A recorrência é expandida no horário do próprio evento, com o jiff, e só depois convertida pro fuso de quem usa: a reunião das 9h continua às 9h depois da virada do horário de verão.
- Regra com parte que o leitor não conhece (`BYSETPOS`, `BYHOUR`, frequência de hora em hora) não inventa ocorrência: o evento fica de fora e sai um aviso dizendo qual foi.
- O download usa o ureq com a TLS do sistema (`native-tls`). O link é segredo: mora só na config, e as mensagens de erro nunca mostram a URL.
- Toda leitura bem-sucedida grava uma cópia em `$XDG_CACHE_HOME/vamo_fogao/agenda.ics`. Sem internet, vale a cópia, com aviso.

## Alternativas consideradas

- **Os crates `icalendar` e `rrule`:** o `rrule` traz o chrono e o chrono-tz, uma segunda biblioteca de data e hora ao lado do jiff que o workspace já usa, com outra base de fusos. O pedaço do ICS que importa aqui cabe em umas 500 linhas testadas, no mesmo estilo do leitor de TOML do acervo.
- **ureq com rustls (o padrão dele):** o rustls traz o ring, cuja licença (ISC e a da OpenSSL antiga) está fora do `deny.toml`. Com a TLS do sistema, tudo continua dentro de MIT e Apache-2.0.
- **Chamar o `curl`:** sem dependência nova, mas o erro vira texto de outro programa, e o app passa a depender de um executável que pode não estar lá.
- **Ler o ICS direto da internet toda vez:** a TUI abriria devagar e não abriria sem rede.

## Consequências

- O core continua sem IO e sem fuso: recebe `Appointment` com data e hora civis, e as regras do tempo e as brechas trabalham só com isso.
- Uma agenda com regra rara pode perder um evento, mas avisa. Quando aparecer um caso de verdade, o leitor ganha o pedaço que falta, com teste.
- Fuso com nome fora da base IANA (o Outlook às vezes manda nomes do Windows) cai no fuso de quem usa, com aviso no log.
- A segunda porta da agenda (o Google por OAuth, opcional) entra atrás da mesma trait `Agenda`.
