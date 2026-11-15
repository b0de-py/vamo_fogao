# 0012. Versionar o formato de exportação com tipos serde e JSON Schema gerado

- **Status:** aceita
- **Data:** 2026-11-14

## Contexto

Na fase 5 os dados saem do app: o log de eventos vai pra um repositório de backup e é lido por outras máquinas, a súmula fica guardada e o MCP vai ler a súmula e a rodada. Quem escreve e quem lê podem estar em versões diferentes do `vamo` (o notebook atualizado, o PC do trampo não). Até aqui o projeto lia JSON e TOML campo a campo, sem `serde` derive, porque os formatos eram de dentro do app ou de outros (a API do Google, a config).

## Decisão

- O formato mora no `vamo-sync`, no módulo `format`, como tipos com `serde` derive: `EventLine` e `MarkLine` (uma linha de JSONL cada), `SummaryDoc` e `WeekDoc`. Os campos são em português, com `deny_unknown_fields`, e a conversão pro domínio passa pelo `FromStr` de cada tipo, que confere de novo.
- Toda linha e todo documento carregam a `versao`. A regra de compatibilidade é uma só: versão nova lê export antigo. Linha de versão maior que a conhecida é recusada com um erro que pede pra atualizar.
- O JSON Schema de cada tipo é gerado dos mesmos tipos pelo `schemars` e fica guardado em `crates/vamo-sync/schema/v{N}/`. Um teste compara o gerado com o guardado: mudar o formato sem subir a `VERSION` quebra a CI. O gerador (`VAMO_GRAVAR_SCHEMA=1`) só escreve arquivo que ainda não existe.
- Os exemplos de cada versão ficam em `crates/vamo-sync/tests/fixtures/v{N}/`, inventados, e um teste lê todos de 1 até a atual. É assim que "versão nova lê export antigo" vira teste, não promessa.

## Alternativas consideradas

- **Escrever o JSON campo a campo, como no resto do app:** o leitor e o escritor viram dois códigos que podem divergir sem ninguém ver, e o schema teria que ser escrito na mão. Num contrato entre versões, o derive é o que garante que os dois lados são o mesmo tipo.
- **Escrever o schema na mão e validar contra ele:** o schema e o código podem divergir; gerar do tipo tira essa chance.
- **Um formato binário (bincode, MessagePack):** menor, mas o backup é um repositório git, e JSONL dá diff legível, um evento por linha.
- **Sem versão, confiando no `deny_unknown_fields`:** o vamo velho recusaria o export novo com um erro sem explicação, e não haveria regra pra quando um campo muda de sentido.

## Consequências

- Duas dependências novas, `serde` (com derive) e `schemars`, as duas MIT ou Apache-2.0. O `serde` já vinha como dependência de outros crates.
- Mudar qualquer tipo do formato é uma decisão visível: versão nova, pasta nova de schema e de exemplos, e a leitura da versão anterior continua no código.
- A súmula guardada no banco e a saída `--json` do `vamo resumo` são o mesmo documento do formato.
