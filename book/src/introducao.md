# O livro do vamo_fogao

O vamo_fogao é um app de tarefas em Rust, feito por um botafoguense pra torcida do Botafogo. Ele vive no terminal (uma CLI e uma TUI), aparece na barra do desktop e conversa com o Claude por MCP. As tarefas cabem numa linha de caderno, concluir uma tarefa é gol, e todo dia abre com uma curiosidade da história do clube, de fonte conferida.

Este livro é pra quem quer entender o código: por que ele tem a forma que tem, por onde começar a ler e que pedaço do Rust aparece em cada parte. Os outros documentos do repositório cuidam do resto:

- o [README](https://github.com/b0de-py/vamo_fogao#readme) conta o que o app faz;
- o [ROADMAP](https://github.com/b0de-py/vamo_fogao/blob/main/ROADMAP.md) diz o que vem por aí;
- as [ADRs](https://github.com/b0de-py/vamo_fogao/tree/main/docs/adr) guardam cada decisão e o porquê dela.

Aqui fica a costura entre tudo isso.

## Pra quem é

Pra quem já escreveu um pouco de Rust e quer ver um projeto inteiro de pé, com teste, doc e CI de verdade. Não é um curso de Rust: quando um conceito aparece, o livro mostra onde ele está no código e por que foi usado ali, e aponta a documentação oficial pra quem quiser ir fundo.

## Em que pé está

O livro cresce junto com o código. Na fase 0 ele tem o esqueleto: este capítulo, o roteiro de leitura, a arquitetura, o Rust que já aparece e o glossário. Cada fase seguinte acrescenta os capítulos dela, no mesmo PR do código.

## Como ler na sua máquina

```sh
cargo install --locked mdbook
mdbook serve book --open
```

O `mdbook serve` recarrega a página sozinho a cada arquivo salvo.
