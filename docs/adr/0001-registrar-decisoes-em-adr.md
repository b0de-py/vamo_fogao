# 0001. Registrar as decisões em ADR

- **Status:** aceita
- **Data:** 2026-11-11

## Contexto

O vamo_fogao é escrito pelo Claude Code e revisado pelo Raul, que aprende Rust lendo o código. Sessões do Claude não lembram umas das outras, e o porquê de uma escolha some rápido quando fica só na conversa ou na mensagem de um commit.

## Decisão

Toda decisão que muda a arquitetura, uma dependência importante, um formato de dado ou uma regra do projeto ganha uma ADR em `docs/adr/`, numerada, no mesmo commit da mudança. O formato é o do [modelo](modelo.md): contexto, decisão, alternativas e consequências.

## Alternativas consideradas

- **Wiki do GitHub:** fica fora do repositório, não passa por revisão de PR e não anda junto com o código.
- **Comentário no código:** bom pro porquê de um trecho, ruim pra decisão que atravessa vários crates.
- **Só a mensagem de commit:** some no histórico e não tem lugar pras alternativas.

## Consequências

- O porquê fica no repositório, revisado junto com o código.
- Cada decisão custa uns minutos de escrita a mais.
- ADR aceita não se edita; mudança de ideia vira ADR nova que substitui a antiga.
