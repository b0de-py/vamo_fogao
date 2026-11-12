# Decisões de arquitetura (ADRs)

Cada escolha que muda o jeito do projeto ganha um registro curto: o contexto, a decisão, o que ficou de fora e o que muda por causa dela. Quem chega depois entende o porquê sem precisar caçar em commit antigo.

| Nº | Decisão | Status |
|----|---------|--------|
| [0001](0001-registrar-decisoes-em-adr.md) | Registrar as decisões em ADR | aceita |
| [0002](0002-arquitetura-hexagonal-em-workspace.md) | Arquitetura hexagonal num workspace Cargo | aceita |
| [0003](0003-licencas.md) | Licenças do código e do acervo | aceita |
| [0004](0004-log-de-eventos-com-ultimo-que-escreveu-ganha.md) | Log de eventos com "o último que escreveu ganha" por campo | aceita |
| [0005](0005-acervo-em-toml-com-a-regua-no-validador.md) | Guardar o acervo em TOML, um fato por arquivo, com a régua no validador | aceita |
| [0006](0006-sqlite-com-migracoes-proprias.md) | Guardar o log em SQLite, com migrações próprias | aceita |
| [0007](0007-tui-com-ratatui-e-porta-propria.md) | Desenhar a TUI com o ratatui, recebendo as tarefas por uma porta própria | aceita |
| [0008](0008-agenda-por-ics-com-leitor-proprio.md) | Ler a agenda pelo link ICS, com leitor próprio e cópia local | aceita |
| [0009](0009-marcacao-da-agenda-fora-do-log.md) | Guardar a marcação das atividades fixas da agenda fora do log de eventos | aceita |

## Como escrever uma

1. Copie o [modelo](modelo.md) pra `NNNN-titulo-curto.md`, com o próximo número livre.
2. Preencha as seções e coloque a ADR no mesmo commit da mudança que ela explica.
3. Acrescente a linha na tabela acima.

ADR aceita não é editada. Se a decisão mudar, uma ADR nova explica a mudança e a antiga passa a dizer "substituída pela NNNN".
