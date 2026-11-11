# A TUI

`vamo tui` abre a interface de terminal: as tarefas do dia, o caderno, o banco e o museu, com o fogo alvinegro no rodapé. Este capítulo mostra como usar e como o código está montado; o porquê das escolhas está na [ADR 0007](https://github.com/b0de-py/vamo_fogao/blob/main/docs/adr/0007-tui-com-ratatui-e-porta-propria.md).

## As telas

| Tecla | Tela | O que mostra |
|-------|------|--------------|
| `1` | Hoje | A escalação do dia, as atrasadas e as fixas, com o placar e a curiosidade do dia. |
| `2` | Caderno | As mesmas tarefas como linhas curtas, do jeito de copiar pro caderno de papel. |
| `3` | Banco | Todas as tarefas em jogo, com filtro. `enter` abre a Ficha da tarefa, com os campos e a súmula. |
| `4` | Museu | O acervo inteiro, por data ou por tema, e a resenha separada dos fatos. |

`tab` e `shift+tab` andam entre as telas, `?` mostra as teclas da tela aberta e `q` sai. Nas listas, `j` e `k` (ou as setas) sobem e descem.

Na Hoje e no Banco, o espaço conclui a tarefa (gol!) e, numa tarefa já concluída, desfaz (o VAR anulou). Numa fixa, o espaço marca o dia. No Banco, `e` escala pra hoje, `b` devolve pro banco, `x` descarta, `n` cria e `/` filtra por área, status, encaixe ou pedaço do título.

## O fogo alvinegro

O fogo do rodapé acompanha o placar do dia: com nada feito, é só um fogo piloto; com tudo feito, as chamas chegam no alto da faixa. Concluir uma tarefa põe a brasa no máximo por um segundo e pisca um GOOOL! por cima.

É o fogo das demos antigas, numa grade de calor (`crates/vamo-tui/src/fire.rs`). A linha de baixo é a brasa. A cada passo, cada célula puxa o calor da de baixo, perde de 0 a 2 pontos e escorrega uma coluna pro lado ao acaso. O calor máximo é a altura da grade, então com a brasa cheia o fogo sobe até o topo, e com metade, até a metade. Cada linha do terminal mostra duas linhas do fogo com o meio-bloco `▀`: a cor da letra é a célula de cima e a do fundo, a de baixo. As cores vão do quase preto ao branco, sem nenhum outro tom.

O acaso vem de um gerador com semente fixa (`SEED`), então o mesmo número de passos dá sempre o mesmo fogo. É o que deixa o fogo ter teste: um snapshot da grade e um teste que confere que a chama sobe com o placar.

O fogo só gasta CPU quando está à vista. O laço principal (`main_loop` em `lib.rs`) dá um passo a cada 80 ms enquanto a animação está ligada e a janela tem foco. Quando o terminal avisa que a janela perdeu o foco, ou com a animação desligada, o laço volta a esperar tecla e o processo fica parado.

## A config

O arquivo `~/.config/vamo_fogao/config.toml` é opcional:

```toml
# jogo (o padrão) ou neutra
linguagem = "neutra"
# false apaga o fogo
animacao = false
```

`vamo --linguagem` e a variável `VAMO_LINGUAGEM` ganham da config. Um campo que o `vamo` não conhece é erro, pra erro de digitação não passar em silêncio.

## No Hyprland: a janela que aparece e some

A ideia é ter o vamo à mão o tempo todo: um atalho mostra a TUI, o mesmo atalho esconde, e o processo segue vivo no meio, com a tela do jeito que ficou. O Hyprland faz isso com um workspace especial, que aparece por cima dos outros e some sem fechar nada.

O arquivo pronto está em [`extras/hyprland/vamo_fogao.conf`](https://github.com/b0de-py/vamo_fogao/blob/main/extras/hyprland/vamo_fogao.conf). Copie pra `~/.config/hypr/` e ponha no fim do `hyprland.conf`:

```text
source = ~/.config/hypr/vamo_fogao.conf
```

Ele faz três coisas:

1. **O atalho.** `bind = SUPER ALT, F, togglespecialworkspace, vamo` mostra e esconde o workspace `special:vamo`. Confira antes com `hyprctl binds` se o atalho está livre; se não estiver, troque.
2. **A primeira abertura.** `workspace = special:vamo, on-created-empty:...` abre o terminal com a TUI quando o workspace aparece vazio. Das outras vezes ele já tem a janela, e o atalho só mostra e esconde. Se você fechar a TUI com `q`, a próxima vez abre de novo.
3. **O tamanho.** A regra de janela deixa a TUI flutuante, centralizada, com 90% da largura e 85% da altura da tela. Pra tela cheia, comente a regra: sozinha no workspace especial, a janela ocupa a tela toda.

A regra acha a janela pela classe `vamo_fogao`, que o terminal recebe na linha de comando (`--class` no Alacritty e no Kitty). Quando o workspace some, a janela perde o foco, o terminal avisa a TUI e o fogo pausa.

## Como o código está montado

- **Estado e desenho.** O `App` (`app.rs`) guarda tudo: a aba, os cursores, a Ficha aberta, a linha de digitar, o fogo. As teclas mudam o estado; o `render` desenha o quadro inteiro a partir dele e não muda nada.
- **Cada tela num módulo.** `today.rs`, `notebook.rs`, `bench.rs`, `card.rs` e `museum.rs` têm o estado da tela (cursor, filtro), as teclas e o desenho. Uma tecla vira uma `Action`: gravar uma mudança, abrir a Ficha, pedir um texto.
- **A porta `Ledger`.** A TUI não abre o banco. Ela pede ao core a mudança (`Projection::finish`, `schedule`...) e grava pela trait `Ledger`, que o binário implementa com a mesma sessão da CLI. Nos testes, um log em memória faz esse papel.
- **O dicionário.** Todo texto de interface sai da `Voice` (`voice.rs`), na linguagem de jogo ou na neutra. A CLI usa o mesmo dicionário.
- **Os testes.** Cada tela é desenhada num terminal de mentira (o `TestBackend` do ratatui) e comparada com um snapshot do insta. Um teste desenha todas as telas no modo neutro e confere que nenhuma palavra do futebol escapa.
