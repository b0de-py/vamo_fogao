# Roadmap

O vamo_fogao anda por fases. Cada fase leva o nome de um ídolo do Botafogo e vira uma issue épica, com as tarefas como sub-issues, uma branch e um PR no fim.

| Fase | O que entra | O que muda pra quem usa | Épico |
|------|-------------|-------------------------|-------|
| 0. Fundação (Nilton Santos) | Workspace, CI, lints, CLAUDE.md, hooks, as primeiras ADRs e o esqueleto do livro. | Nada ainda; as regras de excelência valem desde o primeiro commit. | [#1](https://github.com/b0de-py/vamo_fogao/issues/1) |
| 1. Núcleo e migração (Didi) | Domínio, SQLite, log de eventos, CLI de tarefas, verificador de título, import do banco antigo com a triagem e o formato do acervo com o validador. | O vamo_fogao vira o banco de tarefas. | [#11](https://github.com/b0de-py/vamo_fogao/issues/11) |
| 2. TUI (Garrincha) | Telas Hoje, Caderno, Banco, Ficha e Museu, o tema do r4ul.com.br, o fogo alvinegro, a linguagem de jogo, a curiosidade do dia com os 50 primeiros fatos e a janela que aparece e some. | O app entra no dia a dia. | [#20](https://github.com/b0de-py/vamo_fogao/issues/20) |
| 3. Agenda (Zagallo) | Leitura da agenda por ICS (OAuth opcional), eventos que viram tarefas, o rodapé `vamo:`, brechas, regras do tempo e a caixa pra agendar. | O check-in da manhã acontece sem gastar token. | [#30](https://github.com/b0de-py/vamo_fogao/issues/30) |
| 4. Barra, lembretes e dia de jogo (Jairzinho) | Módulo QML na barra, notificações do Omarchy e o card do dia de jogo com o fato do confronto. | Lembrete vira agenda mais notificação, e dia de jogo ganha card. | [#38](https://github.com/b0de-py/vamo_fogao/issues/38) |
| 5. Resumo e sincronia (Gérson) | Súmula do dia às 4h com os hábitos, export pro backup privado, vários PCs em sincronia e a branch `vamo-entrada`. | A revisão semanal liga. | [#44](https://github.com/b0de-py/vamo_fogao/issues/44) |
| 6. MCP (Túlio) | `vamo mcp` com rmcp, as ferramentas de tarefa, os recursos do dia e da semana e os prompts `weekly-review` e `schedule-inbox`. | O Claude fecha o ciclo sem copiar e colar nada. | [#51](https://github.com/b0de-py/vamo_fogao/issues/51) |
| 7. Pra torcida (Loco Abreu) | Config de exemplo, modo neutro, README de boas-vindas, binários no Releases, pacote no AUR e o modelo de issue pra sugerir curiosidade. | Qualquer botafoguense instala com um comando. | [#58](https://github.com/b0de-py/vamo_fogao/issues/58) |
| Extra (Monumental) | MCP remoto no VPS e a página do vamo_fogao no r4ul.com.br. | O Claude fala com o vamo_fogao direto, sem passar pelo git. | [#64](https://github.com/b0de-py/vamo_fogao/issues/64) |
| Sempre: acervo | Fatos novos por PR, cada um com duas fontes, até ter um "Neste dia" pra cada dia do ano. | Cada dia abre com uma história nova. | [#67](https://github.com/b0de-py/vamo_fogao/issues/67) |

## Os codinomes

- **Nilton Santos**, a Enciclopédia do Futebol: a fundação que sabe tudo antes de a bola rolar.
- **Didi**, o meia que organizava o jogo: o núcleo.
- **Garrincha**, a Alegria do Povo: a fase em que o app ganha cara e graça.
- **Zagallo**, o Formiguinha que ocupava todos os espaços do campo: as brechas da agenda.
- **Jairzinho**, o Furacão da Copa de 1970: a fase que aparece em todo canto.
- **Gérson**, o Canhotinha de Ouro dos lançamentos longos: o dado que vai longe.
- **Túlio Maravilha**, o artilheiro: o gol que fecha o ciclo.
- **Loco Abreu**, o da cavadinha: o app vai pra galera.
- **Monumental**, o estádio de Buenos Aires onde veio a Libertadores de 2024.
