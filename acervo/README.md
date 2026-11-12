# Acervo

O museu de bolso do vamo_fogao: fatos da história do Botafogo, cada um com as suas fontes. É daqui que sai a curiosidade do dia.

- `fatos/`: um fato por arquivo.
- `resenha/`: a zoeira, absurda de cara e com selo próprio. Nunca se mistura com fato.

## A régua

1. Duas fontes independentes, de domínios diferentes. A Wikipédia em qualquer idioma conta como uma só.
2. Ordem de confiança: jornal da época (a Hemeroteca Digital da Biblioteca Nacional) e documento do clube; depois livro, RSSSF e imprensa esportiva. A Wikipédia serve pra achar fonte, não pra fechar.
3. Só o verificado aparece no app. O que ainda não tem as duas fontes fica `a-conferir`, com a nota do que falta.
4. Se as fontes divergem, o texto leva só o que bate, e a divergência fica nas notas.
5. Recorde tem data, porque recorde cai. O texto diz quando valia: "Até a Copa de 2026, ...".
6. Zoeira tem que ser absurda de cara. Time de velociraptores em 4000 a.C. passa; placar inventado de 1960, não.

O `vamo acervo validar` confere o que dá pra conferir de máquina (as fontes, a data, o formato de cada campo) e roda na CI em todo PR. O resto fica com quem revisa: se as duas fontes são independentes de verdade (uma pode copiar a outra) e se o texto diz só o que elas dizem.

## O formato

Cada fato é um arquivo TOML. O nome do arquivo é a id do fato: letra minúscula sem acento, número e hífen, como `goleada-de-24-a-0-no-mangueira.toml`.

```toml
texto = "24 x 0 no Mangueira, pelo Campeonato Carioca: a maior goleada da história do Botafogo."
data = "1909-05-30"
tipo = "recorde"
status = "verificado"
tags = ["adversario:mangueira"]
notas = "As fontes divergem sobre o local, então o texto fica sem ele."

[[fontes]]
nome = "Lance!"
url = "https://www.lance.com.br/lancepedia/maior-goleada-da-historia-do-botafogo.html"

[[fontes]]
nome = "Wikipédia em inglês: Sport Club Mangueira"
url = "https://en.wikipedia.org/wiki/Sport_Club_Mangueira"
```

| Campo | Obrigatório | O que é |
|-------|-------------|---------|
| `texto` | sim | O fato, numa linha, com até 200 caracteres. |
| `status` | sim | `verificado` ou `a-conferir`. |
| `data` | no recorde | `AAAA`, `AAAA-MM` ou `AAAA-MM-DD`, com a precisão que as fontes dão. |
| `ate` | não | O fim de um período, como `data = "1977"` com `ate = "1978"`. |
| `tipo` | não | `fato` (o padrão) ou `recorde`. |
| `tags` | não | `epoca:`, `idolo:`, `titulo:`, `adversario:` ou `esporte:` (quando não é futebol), seguido de um nome em minúsculas e com hífen, como `adversario:atletico-mg`. O `adversario:` é o campo do confronto: em dia de jogo contra esse time, o fato aparece no quadro da tela Hoje. Use o nome como as tabelas escrevem, sem acento (`sao-paulo`), ou o começo dele (`vasco` vale pro `Vasco da Gama`). |
| `notas` | não | A divergência entre as fontes ou o que falta pra conferir. |
| `[[fontes]]` | pelo menos uma | Cada fonte com `nome` e, se estiver na internet, `url`. Fonte sem link (livro, documento) conta pelo nome. |

Na resenha, o arquivo só tem `texto` e, se precisar, `notas`.

## Pra conferir

```sh
cargo run -- acervo validar
```

Se algum arquivo não passa, a saída diz qual, em que linha e o que fazer.

## Licença

Os textos desta pasta estão sob a [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/deed.pt-br), a mesma licença da Wikipédia ([ADR 0003](../docs/adr/0003-licencas.md)). Fato histórico não tem dono: a licença cobre o texto que conta o fato. Quem reaproveitar mantém a licença e dá o crédito ao vamo_fogao.
