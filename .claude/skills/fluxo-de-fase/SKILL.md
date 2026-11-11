---
name: fluxo-de-fase
description: Fluxo de GitHub do vamo_fogao pra trabalhar numa fase do ROADMAP.md. A fase é uma issue épica com as tarefas como sub-issues; o trabalho vai numa branch feature/fase-N, com um commit por tarefa fechando a sub-issue, push depois de cada commit e um PR no fim, sem merge. Use quando pedirem pra começar uma fase, criar as issues de uma fase, subir uma tarefa, abrir o PR de uma fase ou falarem do fluxo do projeto, mesmo sem essas palavras ("bora pra fase 2", "sobe essa task", "fecha a fase 1").
---

# Fluxo de fase do vamo_fogao

Cada fase do `ROADMAP.md` vira objetos no GitHub. O merge é sempre manual, feito pelo Raul no próprio GitHub.

```
issue épica da fase          branch feature/fase-N          PR "Fase N: Nome (Codinome)"
+ uma sub-issue por tarefa → um commit por tarefa        →  merge manual do Raul
                             push depois de cada commit
```

Pré-requisito: `gh` autenticado (`gh auth status`). Sem ele não dá pra criar issue nem PR; peça pro Raul rodar `gh auth login`.

## 1. Abrir a fase

As fases 0 a 7, o Extra e o Acervo já têm épico e sub-issues. Confira antes de criar qualquer coisa:

```sh
gh issue list --label épico --state all
gh api repos/b0de-py/vamo_fogao/issues/<épico>/sub_issues --jq '.[] | "#\(.number) \(.title)"'
```

Se a fase ainda não tiver issues:

1. Crie o épico com o objetivo, o que muda pra quem usa e quando a fase acaba, com as labels `épico` e `fase-N`.
2. Crie uma issue por tarefa, com a label `fase-N` e o corpo no modelo "Tarefa" (O que é / Pronto quando), e pendure cada uma no épico como sub-issue. A API pede o `id` da issue, que não é o número:

   ```sh
   id=$(gh api repos/b0de-py/vamo_fogao/issues/<número> --jq .id)
   gh api repos/b0de-py/vamo_fogao/issues/<épico>/sub_issues -F sub_issue_id="$id"
   ```

Não invente tarefa. Se a fase não estiver quebrada em tarefas no épico ou no `ROADMAP.md`, avise o Raul em vez de preencher com suposição.

## 2. Trabalhar na fase

```sh
git switch main && git pull
git switch -c feature/fase-N
```

Pra cada tarefa, na ordem das sub-issues:

1. Implemente só o escopo da tarefa, com testes e doc.
2. Rode `./scripts/checar.sh`. Não commite trabalho pela metade.
3. Faça um commit dedicado, que fecha a sub-issue quando o PR entrar:

   ```
   <Título no imperativo, em português, até 72 caracteres>

   <Por que a mudança existe e o que ela muda.>

   Closes #<sub-issue>
   ```

4. Push logo em seguida: `git push -u origin feature/fase-N`.

Se uma tarefa crescer além do escopo, quebre em sub-issues novas em vez de misturar tudo num commit. Decisão de arquitetura ganha ADR em `docs/adr/` no mesmo commit.

## 3. Fechar a fase

Abra o PR pra `main` com o título "Fase N: Nome (Codinome)" e o corpo no modelo de PR, com um `Closes #` pro épico e pra cada sub-issue. Mande o link pro Raul e pare aí: o merge é dele.

## O que nunca fazer

- Push direto na `main`, merge de PR, force push.
- Apagar branch, issue ou label sem o Raul pedir.
- Colocar dado pessoal no repositório, que é público.
