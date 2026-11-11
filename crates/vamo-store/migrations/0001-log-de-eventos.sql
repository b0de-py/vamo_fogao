-- 0001: o log de eventos.
--
-- Cada linha é uma mudança numa tarefa, e o estado das tarefas é calculado a
-- partir daqui (ADR 0004). Os valores ficam como o próprio tipo do domínio
-- escreve e lê de volta: `T-041`, `brecha`, `concluída`. Assim dá pra olhar o
-- banco no `sqlite3` e entender o que está ali.

CREATE TABLE events (
    -- A ordem de gravação. O log é lido nessa ordem.
    seq     INTEGER PRIMARY KEY,
    -- O ID do evento, um UUID v7. O UNIQUE é o que faz o mesmo evento não
    -- entrar duas vezes.
    id      TEXT NOT NULL UNIQUE,
    -- O instante em RFC 3339, em UTC. Não ordene por esta coluna no SQL: as
    -- frações de segundo têm tamanho variável e embaralham a ordem do texto.
    -- Quem ordena por instante é a projeção.
    at      TEXT NOT NULL,
    machine TEXT NOT NULL,
    task    TEXT NOT NULL,
    -- O que mudou. A criação preenche título, área, encaixe e status; as
    -- outras mudanças preenchem só a coluna delas.
    kind    TEXT NOT NULL
            CHECK (kind IN ('created', 'title', 'area', 'fit', 'status', 'notes')),
    title   TEXT,
    area    TEXT,
    fit     TEXT,
    status  TEXT,
    notes   TEXT
) STRICT;

-- Pra ver o histórico de uma tarefa sem varrer o log inteiro.
CREATE INDEX events_by_task ON events (task);
