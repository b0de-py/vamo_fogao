# O acervo

O museu de bolso: os fatos da história do Botafogo que viram a curiosidade do dia. A régua (duas fontes independentes, recorde com data, só o verificado aparece) e o formato dos arquivos estão no [README do acervo](https://github.com/b0de-py/vamo_fogao/tree/main/acervo); o porquê, na [ADR 0005](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0005-acervo-em-toml-com-a-regua-no-validador.md). Este capítulo mostra como o código confere a régua.

## O caminho de um arquivo

```text
vamo acervo validar
  └─ crates/vamo_fogao/src/acervo.rs     lê acervo/fatos/ e acervo/resenha/
       └─ Fact::from_toml(id, texto)     crates/vamo-acervo/src/fact.rs
            ├─ Ok(Fact)                  conta como verificado ou a conferir
            └─ Err(Error)                vira uma linha "arquivo:linha: problema"
```

O `vamo-acervo` é puro como o core: recebe o texto de cada arquivo e devolve o fato ou o erro. Quem lê a pasta, ordena os arquivos e junta o relatório é o binário. O mesmo comando roda no job Acervo da CI, então um PR com fato fora da régua não passa.

## Ler TOML com a linha de cada campo

O jeito mais curto de ler TOML em Rust é o serde, que preenche uma struct inteira de uma vez. Aqui ele ficou de fora por dois motivos: as mensagens de erro dele vêm em inglês, e quem escreve um fato precisa saber em que linha está o problema.

`crates/vamo-acervo/src/document.rs` usa o leitor de baixo nível do crate `toml`, o `DeTable`, que guarda pra cada chave e cada valor o pedaço do texto de onde ele veio (o *span*, um intervalo de bytes). A linha sai contando as quebras de linha antes do começo do span:

```rust
fn line_of(text: &str, offset: usize) -> usize {
    text.get(..offset)
        .map_or(1, |before| before.matches('\n').count() + 1)
}

assert_eq!(line_of("texto = \"x\"\ndata = 3", 12), 2);
```

A struct `Table<'a>` empresta a tabela já lida e o texto do arquivo. O `'a` diz ao compilador que os `&str` que ela devolve vivem tanto quanto o texto original: nada é copiado até o valor virar um tipo do domínio. Cada leitura (`string`, `strings`, `tables`) devolve o valor junto com a linha, e o erro sai preso nela com `Error::at(linha, erro)`.

Campo desconhecido também é erro (`Table::only`). Um `fonte` no lugar de `fontes` quase sempre é digitação, e calar sobre ele deixaria o fato sem fonte sem ninguém perceber.

## Os tipos

Como no core, cada valor vira tipo na leitura:

| Tipo | No arquivo | O que garante |
|------|------------|---------------|
| `Slug` | o nome do arquivo, o valor das tags | Minúsculas sem acento, número e hífen. |
| `ShortText` | `texto` | Uma linha, até 200 caracteres (contando caractere, não byte). |
| `FactDate` | `data`, `ate` | `AAAA`, `AAAA-MM` ou `AAAA-MM-DD`, com a data existindo no calendário. |
| `Kind` | `tipo` | `fato` ou `recorde`. |
| `Verification` | `status` | `verificado` ou `a-conferir`. |
| `Tag` | `tags` | Um enum com dados: `Idol(Slug)`, `Opponent(Slug)` e assim por diante. |
| `Source` | `[[fontes]]` | Nome não vazio e, se tiver, um link `http(s)` com domínio. |

O `FactDate` guarda o mês e o dia como `Option`, porque a fonte nem sempre dá o dia. O `derive(Ord)` faz a ordem do tempo sair de graça: compara o ano, depois o mês, depois o dia, e `None` vem antes de qualquer `Some`, então `1909` vem antes de `1909-05`, que vem antes de `1909-05-30`.

## A régua das fontes

Duas fontes são independentes quando vêm de **origens** diferentes. Em `crates/vamo-acervo/src/source.rs`, a origem de uma fonte com link é o domínio registrável: o pedaço do endereço que alguém registra e controla.

```rust,ignore
let keep = match labels.as_slice() {
    [.., second, country] if country.len() == 2 && SECOND_LEVEL.contains(second) => 3,
    _ => 2,
};
```

O slice pattern `[.., second, country]` pega os dois últimos pedaços do domínio, ignorando o resto. Se o penúltimo é um rótulo genérico (`com`, `org`, `gov`...) antes de um país, o domínio registrável tem três partes (`lance.com.br`); se não, duas (`globo.com`, que junta `ge.globo.com` e `oglobo.globo.com`). Por cima disso, todo domínio da Wikimedia vira `wikipedia.org`, e é assim que a Wikipédia em qualquer idioma conta como uma fonte só. Fonte sem link, como livro, tem o próprio nome como origem.

A conferência fica em `check_sources`, no `fact.rs`: todo fato precisa de pelo menos uma fonte, e o verificado precisa de duas origens diferentes. As origens vão para um `BTreeSet`, que descarta repetidas, e basta contar.

A régua não enxerga tudo. Duas matérias de veículos diferentes podem ter copiado uma da outra, e só quem revisa o PR percebe. O validador cuida do que dá pra conferir de máquina; o VAR, do resto.

## A curiosidade do dia

Em `crates/vamo-acervo/src/draw.rs`. A curiosidade é um "sorteio" que não sorteia: o fato sai da data, então o mesmo dia mostra o mesmo fato em qualquer PC, sem guardar nada nem sincronizar nada.

1. Só fato `verificado` entra.
2. Se algum fato aconteceu neste dia e mês, em qualquer ano, ele tem prioridade: é o "Neste dia". Com mais de um, cada ano mostra um.
3. Sem "Neste dia", vale o rodízio. Os fatos, em ordem de id, são embaralhados uma vez, sempre do mesmo jeito (um Fisher-Yates com semente fixa), e cada dia anda uma casa. Um fato só repete depois que todos passaram.

A casa do dia é o número de dias desde 1º de janeiro de 1970, contado pelo algoritmo `days_from_civil`, de Howard Hinnant. O acervo não lê o relógio: quem chama passa o `Day` de hoje.

O acervo vai dentro do binário. O `build.rs` do `vamo_fogao` transforma cada `.toml` de `acervo/` num `include_str!`, então quem instala o `vamo` tem a curiosidade sem precisar da pasta. É o `vamo curiosidade` no terminal, e o quadro da tela Hoje na TUI, onde a tecla `r` troca o fato pela resenha.
