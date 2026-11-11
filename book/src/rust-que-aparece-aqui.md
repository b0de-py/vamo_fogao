# O Rust que aparece aqui

Um mapa dos pedaços do Rust que o vamo_fogao usa, com o porquê de cada um e onde olhar no código. Não substitui [o livro oficial](https://doc.rust-lang.org/book/): a ideia é ligar o conceito ao lugar onde ele trabalha.

## O projeto

### Workspace e herança de configuração

O `Cargo.toml` da raiz não é de um crate: é de um workspace, um conjunto de crates que compilam juntos e dividem o mesmo `Cargo.lock` e a mesma pasta `target/`. Os metadados (`[workspace.package]`), as versões das dependências (`[workspace.dependencies]`) e os lints (`[workspace.lints]`) ficam ali, e cada crate herda com `campo.workspace = true`. Uma regra muda num lugar e vale pra todos.

Referência: [Workspaces, no livro do Cargo](https://doc.rust-lang.org/cargo/reference/workspaces.html).

### Lints como contrato

O clippy no modo pedante, `unsafe` proibido com `forbid` e doc obrigatória em item público. Na CI, todo aviso vira erro (`-D warnings`). Dois detalhes:

- `forbid` é mais forte que `deny`: nem um `#[allow]` local consegue abrir exceção.
- Quando uma regra precisa ser suspensa num ponto, o `#[allow]` vem com o motivo: `#[allow(clippy::nome_do_lint, reason = "...")]`. O lint `allow_attributes_without_reason` cobra isso.

## Tipos

### Newtypes que validam na construção

Um *newtype* é uma struct com um campo só, como `pub struct Area(String)`. Com o campo privado, o único jeito de ter uma `Area` é pelo construtor, e o construtor confere o valor. Daí pra frente, o tipo é a garantia: uma função que recebe `Area` não precisa conferir de novo.

Onde olhar: `TaskId`, `Area` e `NotebookTitle` no `vamo-core`; `Slug`, `ShortText` e `FactDate` no `vamo-acervo`. O capítulo [O domínio](o-dominio.md) conta a ideia com mais calma.

### `FromStr` e `Display`, ida e volta

Implementar `FromStr` dá ao tipo o método `parse`; implementar `Display` dá o `to_string`. Os dois juntos fazem o texto virar tipo e voltar, e o mesmo texto serve pra tela, pro banco e pra linha de comando:

```rust
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
struct Gols(u8);

impl FromStr for Gols {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let number = text.strip_suffix(" gols").ok_or("faltou o \"gols\"")?;
        number.parse().map(Gols).map_err(|_| format!("`{number}` não é número"))
    }
}

impl fmt::Display for Gols {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} gols", self.0)
    }
}

let gols: Gols = "24 gols".parse().unwrap();
assert_eq!(gols, Gols(24));
assert_eq!(gols.to_string(), "24 gols");
```

O clap usa o mesmo `FromStr`: é por isso que `vamo add --encaixe foco` já chega no código como `Fit::Focus`, e um valor errado mostra a mensagem do domínio.

### `NonZeroU32` e o nicho

O `TaskId` guarda o número num `NonZeroU32`, um inteiro que não pode ser zero. Além de proibir o `T-000`, isso deixa um espaço vago que o compilador aproveita: um `Option<TaskId>` usa o zero pra representar o `None` e ocupa os mesmos 4 bytes de um `TaskId`.

```rust
use std::num::NonZeroU32;

assert_eq!(size_of::<Option<NonZeroU32>>(), size_of::<u32>());
assert!(NonZeroU32::new(0).is_none());
```

### Enums com dados

Um enum do Rust carrega dados em cada variante. `Change`, no log de eventos, tem uma variante por campo que pode mudar, cada uma com o valor novo; `Tag`, no acervo, tem uma variante por tipo de tag, cada uma com o seu `Slug`. O `match` obriga a tratar todas as variantes, então uma variante nova faz o compilador apontar cada lugar que precisa saber dela.

### `derive(Ord)` e a ordem dos campos

O `#[derive(PartialOrd, Ord)]` compara os campos na ordem em que estão declarados, como um dicionário compara letras. O vamo_fogao usa isso de propósito em dois lugares:

- o `Stamp` da projeção, com `at` antes de `id`: o evento mais recente ganha e, no empate, decide o ID;
- o `FactDate` do acervo, com ano, mês e dia: `None` vem antes de qualquer `Some`, então `1909` vem antes de `1909-05-30`.

## Erros

### Erro se trata, não se ignora

`unwrap()` derruba o programa se o valor não estiver lá. Fora dos testes, o clippy proíbe. O caminho é devolver um `Result` e propagar com `?`, que repassa o erro pra quem chamou:

```rust
use std::num::ParseIntError;

fn parse_number(text: &str) -> Result<u32, ParseIntError> {
    let number = text.trim().parse::<u32>()?;
    Ok(number)
}

assert_eq!(parse_number(" 41 "), Ok(41));
assert!(parse_number("quarenta e um").is_err());
```

### Um enum de erro por crate, com o `thiserror`

Cada crate de biblioteca tem o seu `Error`, um enum com uma variante por coisa que pode dar errado. O `thiserror` escreve o `Display` a partir de um atributo, com os campos da variante no texto:

```rust,ignore
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("não tem nenhuma tarefa {id}")]
    UnknownTask { id: TaskId },
}
```

O binário junta os erros de todos os crates com o `color-eyre`, que monta o relatório (sem vermelho, com o tema do app) e deixa acrescentar contexto com `wrap_err`.

### `Box` num enum recursivo

O `Error` do acervo tem uma variante que embrulha outro erro com a linha onde ele aconteceu: `At { line, error: Box<Error> }`. Um enum não pode conter a si mesmo direto, porque o tamanho dele seria infinito; o `Box` guarda o erro de dentro no heap, e o enum fica com o tamanho de um ponteiro naquele campo.

## Controle de fluxo

### Slice patterns

Um `match` sobre uma fatia olha pro formato da lista inteira de uma vez, dando nome aos pedaços:

```rust
fn list(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [rest @ .., last] => format!("{} e {last}", rest.join(", ")),
    }
}

assert_eq!(list(&["texto", "data", "status"]), "texto, data e status");
```

`[rest @ .., last]` casa com uma lista de qualquer tamanho a partir de um, chama o último de `last` e o resto de `rest`. Onde olhar: `error.rs` e `date.rs` no `vamo-acervo`, e o `[.., second, country]` do domínio registrável em `source.rs`.

### `let-else`

Quando o caminho feliz depende de um padrão casar, o `let-else` dá nome ao valor e manda o resto embora logo no começo:

```rust
fn first_word(text: &str) -> Result<&str, String> {
    let Some(word) = text.split_whitespace().next() else {
        return Err("o título está vazio".to_owned());
    };
    Ok(word)
}

assert_eq!(first_word("Ligar pro encanador"), Ok("Ligar"));
```

O bloco do `else` precisa sair da função (ou do laço), então depois do `let-else` o código segue sem um nível a mais de indentação. Aparece no `title.rs`, no `import.rs` e no `document.rs`.

### `let` encadeado

Desde a edição 2024, um `if let` ou `while let` pode juntar padrões e condições com `&&`. O corte do título do caderno, em `title.rs`, tira palavra fraca do fim enquanto ela existir:

```rust,ignore
while let [rest @ .., last] = kept
    && !rest.is_empty()
    && WEAK_ENDINGS.contains(&last.to_lowercase().as_str())
{
    kept = rest;
}
```

## Traits e empréstimos

### Traits como portas, com tipo associado

O core declara o que precisa como uma trait, e cada adaptador implementa. A `EventStore`, em `port.rs`, tem um tipo associado, `type Error`: cada implementação escolhe o próprio tipo de erro, e o core não precisa conhecer erro de SQLite. Detalhes no capítulo [O log de eventos](o-log-de-eventos.md).

### `FromIterator`: montar com `collect`

Implementar `FromIterator<&Event>` pra `Projection` faz `events.iter().collect()` devolver a projeção pronta. É o mesmo mecanismo que transforma um iterador num `Vec` ou num `BTreeSet`.

### Lifetimes: emprestar em vez de copiar

A `Table<'a>` do acervo, em `document.rs`, guarda referências pro TOML já lido. O `'a` amarra essas referências ao texto original: o compilador garante que a `Table` não vive mais que o texto, e nenhum campo é copiado até virar um tipo do domínio.

## Testes

### Teste de propriedade

Em vez de escrever cada exemplo, o proptest gera centenas de entradas e confere uma propriedade que vale pra todas. Quando acha uma que quebra, encolhe a entrada até o menor caso que ainda falha. Onde olhar: a projeção (a ordem de chegada não importa), o título do caderno (a sugestão de corte é um título válido) e o acervo (data que vai e volta igual, texto qualquer que nunca derruba o leitor).

### Snapshots

O insta guarda a saída esperada num arquivo `.snap`, ao lado do teste, e avisa quando ela muda. A ajuda do `vamo`, a triagem do import e o relatório do validador do acervo são conferidos assim, em `crates/vamo_fogao/tests/snapshots/`.

### Teste do binário de verdade

Os testes em `crates/vamo_fogao/tests/cli.rs` rodam o executável compilado, num processo separado, com a casa e o diretório atual numa pasta temporária. O Cargo entrega o caminho do executável na variável `CARGO_BIN_EXE_vamo`, lida com `env!` em tempo de compilação. É o teste mais próximo de quem usa: confere a saída, o código de saída e as mensagens de erro.

## Chegando nas próximas fases

- **A TUI** (fase 2): telas desenhadas a cada quadro a partir do estado, com snapshot de cada uma.
- **async** (fase 6): o servidor MCP roda em cima do tokio.
