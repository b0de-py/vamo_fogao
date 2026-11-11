# 0003. Licenças do código e do acervo

- **Status:** aceita
- **Data:** 2026-11-11

## Contexto

O repositório é público e feito pra torcida usar e contribuir. Ele tem dois tipos de conteúdo: o código em Rust e, a partir da fase 1, os textos do acervo, com os fatos da história do Botafogo. Muita pesquisa sobre a história do clube passa pela Wikipédia, que é licenciada em CC BY-SA 4.0.

## Decisão

- **Código:** `MIT OR Apache-2.0`, à escolha de quem usa. É o padrão do ecossistema Rust. Os textos ficam em `LICENSE-MIT` e `LICENSE-APACHE`.
- **Textos do acervo:** CC BY-SA 4.0, a mesma licença da Wikipédia. Quando o acervo chegar, a pasta dele traz o aviso.
- **Contribuições** entram sob as mesmas licenças, sem documento extra pra assinar.

Fato histórico, em si, não tem dono. O que a licença do acervo cobre é o texto que conta o fato.

## Alternativas consideradas

- **Só MIT no código:** a Apache-2.0 acrescenta uma concessão explícita de patentes, e oferecer as duas é o costume do Rust.
- **GPL no código:** obrigaria quem reaproveitar o código a abrir o seu; pra um app de torcida, a liberdade de reaproveitar pesa mais.
- **CC BY 4.0 no acervo:** mais permissiva, mas não aceitaria texto adaptado da Wikipédia, que exige compartilhar pela mesma licença.

## Consequências

- Qualquer dependência nova precisa ter licença compatível; o `cargo deny` confere.
- Texto adaptado da Wikipédia pode entrar no acervo com a atribuição.
- Quem usa o acervo em outro projeto mantém a CC BY-SA 4.0 e dá o crédito.
