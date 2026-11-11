# 0005. Guardar o acervo em TOML, um fato por arquivo, com a régua no validador

- **Status:** aceita
- **Data:** 2026-11-11

## Contexto

O acervo é a parte do vamo_fogao que mais recebe contribuição: fatos novos entram por PR, de quem pesquisa e da torcida, e o Raul revisa cada um antes do merge, como o VAR. A curiosidade do dia só vale se o fato for verdade, então a régua (duas fontes independentes, data em recorde, só o verificado aparece) precisa ser conferida em todo PR, sem depender de alguém lembrar. E o formato tem que ser fácil de escrever à mão e de revisar num diff.

## Decisão

- **Um fato por arquivo TOML**, em `acervo/fatos/`, e a zoeira em `acervo/resenha/`. O nome do arquivo é a id do fato, um slug (minúsculas, número e hífen), então a id é única por construção.
- **A régua mora no crate `vamo-acervo`**, puro como o core: recebe o texto do arquivo e devolve o `Fact` ou o erro. O fato só nasce depois de passar na régua.
- **Independência pelo domínio registrável**: duas fontes contam como independentes quando vêm de domínios diferentes (`lance.com.br` e `trivela.com.br`). Subdomínio não conta (`ge.globo.com` e `oglobo.globo.com` são o mesmo), e a Wikipédia, em qualquer idioma ou espelho, é uma fonte só. Fonte sem link, como livro, conta pelo nome.
- **O TOML é lido campo a campo**, sem o serde, pra cada erro sair em português e com a linha.
- **`vamo acervo validar`** lê a pasta, passa cada arquivo pela régua e lista todos os problemas, no formato `arquivo:linha`. Roda no job Acervo da CI e no `./scripts/checar.sh`.

## Alternativas consideradas

- **Um arquivo só com todos os fatos:** dois PRs de fato novo brigariam pelo mesmo arquivo, e o diff de um fato se perderia no meio dos outros.
- **YAML ou JSON:** YAML tem armadilha de indentação e de tipo (o `no` que vira falso); JSON não aceita comentário e enche o texto de aspas e chaves. TOML é o formato que o Cargo já usa no projeto.
- **Markdown com cabeçalho de metadados:** mais bonito de ler, mas são dois formatos num arquivo só e mais código pra ler.
- **Ler com o serde:** menos código, mas as mensagens de erro viriam em inglês e sem a linha certa de cada campo.
- **A lista completa de sufixos públicos (o crate `psl`):** acertaria todo domínio do mundo, mas é uma dependência grande pra um caso que uma lista curta resolve. A aproximação erra pro lado da cautela: num sufixo que ela não conhece, dois sites parecem o mesmo e o fato pede mais uma fonte.
- **Conferir os links no PR:** depende da rede e do humor de cada site, e travaria PR por motivo que não é do PR. Fica pra uma checagem semanal separada.

## Consequências

- A CI barra fato verificado com fonte única, fonte toda do mesmo domínio, recorde sem data, campo desconhecido e valor fora do formato.
- Duas matérias de veículos diferentes que copiam uma da outra passam na régua. Independência de verdade continua sendo trabalho de quem revisa.
- Fato `a-conferir` pode entrar com uma fonte só: é a fila do que ainda falta pesquisar, e não aparece no app.
- Mudar o formato (campo novo, tag nova) é mudar o `vamo-acervo` e os testes dele no mesmo PR.
