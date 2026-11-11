# O Rust que aparece aqui

Um mapa dos pedaços do Rust que o vamo_fogao usa, com o porquê de cada um e onde olhar no código. Não substitui [o livro oficial](https://doc.rust-lang.org/book/): a ideia é ligar o conceito ao lugar onde ele trabalha.

## Já no código

### Workspace e herança de configuração

O `Cargo.toml` da raiz não é de um crate: é de um workspace, um conjunto de crates que compilam juntos e dividem o mesmo `Cargo.lock` e a mesma pasta `target/`. Os metadados (`[workspace.package]`), as versões das dependências (`[workspace.dependencies]`) e os lints (`[workspace.lints]`) ficam ali, e cada crate herda com `campo.workspace = true`. Uma regra muda num lugar e vale pra todos.

Referência: [Workspaces, no livro do Cargo](https://doc.rust-lang.org/cargo/reference/workspaces.html).

### Lints como contrato

O clippy no modo pedante, `unsafe` proibido com `forbid` e doc obrigatória em item público. Na CI, todo aviso vira erro (`-D warnings`). Dois detalhes:

- `forbid` é mais forte que `deny`: nem um `#[allow]` local consegue abrir exceção.
- Quando uma regra precisa ser suspensa num ponto, o `#[allow]` vem com o motivo: `#[allow(clippy::nome_do_lint, reason = "...")]`. O lint `allow_attributes_without_reason` cobra isso.

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

A partir da fase 1, cada crate de biblioteca tem o seu enum de erro, feito com o `thiserror`, e o binário junta tudo com o `color-eyre` pra mostrar uma mensagem legível.

### Slice patterns no `main`

O `main` do `vamo` decide o que fazer olhando pra lista de argumentos inteira de uma vez:

```rust,ignore
match args.as_slice() {
    [] => print_help(),
    [arg] => /* um argumento só */,
    _ => usage_error("argumentos demais"),
}
```

`[]` casa com a lista vazia, `[arg]` com exatamente um elemento (já dando nome a ele) e `_` com o resto. É mais claro que um monte de `if args.len() == ...`.

Referência: [padrões de slice, na referência do Rust](https://doc.rust-lang.org/reference/patterns.html#slice-patterns).

### `OsString` em vez de `String`

Argumento de linha de comando não é obrigatoriamente UTF-8. `std::env::args()` entra em pânico se receber um que não seja; `std::env::args_os()` devolve `OsString`, que aceita qualquer coisa. O `vamo` usa a segunda e converte com `to_str()` só na hora de comparar, então argumento estranho vira erro de uso, não pânico.

### Teste do binário de verdade

Os testes em `crates/vamo_fogao/tests/cli.rs` rodam o executável compilado, num processo separado. O Cargo entrega o caminho dele na variável `CARGO_BIN_EXE_vamo`, lida com `env!` em tempo de compilação. É o teste mais próximo de quem usa: confere a saída, o código de saída e as mensagens de erro.

## Chegando nas próximas fases

- **Tipos que validam na construção** (fase 1): `TaskId` e `NotebookTitle` só existem se forem válidos. Um construtor que devolve `Result` e campos privados garantem isso.
- **Ownership no log de eventos** (fase 1): os eventos são imutáveis e a projeção lê o log por referência, sem copiar.
- **Traits como portas** (fase 1 em diante): o core declara o que precisa, e cada adaptador implementa.
- **Testes de propriedade** (fase 1): o proptest gera milhares de entradas e confere uma propriedade, como "ler o que foi escrito devolve o mesmo valor".
- **Snapshots** (fase 2): o insta guarda a saída esperada de uma tela num arquivo e avisa quando ela muda.
- **async** (fase 6): o servidor MCP roda em cima do tokio.
