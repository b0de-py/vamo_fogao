# Decisões de arquitetura (ADRs)

Cada escolha que muda o jeito do projeto ganha um registro curto: o contexto, a decisão, o que ficou de fora e o que muda por causa dela. Quem chega depois entende o porquê sem precisar caçar em commit antigo.

| Nº | Decisão | Status |
|----|---------|--------|
| [0001](0001-registrar-decisoes-em-adr.md) | Registrar as decisões em ADR | aceita |
| [0002](0002-arquitetura-hexagonal-em-workspace.md) | Arquitetura hexagonal num workspace Cargo | aceita |
| [0003](0003-licencas.md) | Licenças do código e do acervo | aceita |

## Como escrever uma

1. Copie o [modelo](modelo.md) pra `NNNN-titulo-curto.md`, com o próximo número livre.
2. Preencha as seções e coloque a ADR no mesmo commit da mudança que ela explica.
3. Acrescente a linha na tabela acima.

ADR aceita não é editada. Se a decisão mudar, uma ADR nova explica a mudança e a antiga passa a dizer "substituída pela NNNN".
