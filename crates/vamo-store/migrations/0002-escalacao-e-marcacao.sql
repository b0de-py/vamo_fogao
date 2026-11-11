-- 0002: o dia da escalação e a marcação das tarefas recorrentes.
--
-- Duas mudanças novas no log: `day` (a tarefa foi escalada pra um dia, ou
-- saiu da escalação) e `check` (uma recorrente foi marcada num dia). O
-- SQLite não deixa trocar o CHECK de uma coluna, então a tabela é refeita
-- com as colunas novas e os eventos antigos são copiados, na mesma ordem.

CREATE TABLE events_new (
    seq     INTEGER PRIMARY KEY,
    id      TEXT NOT NULL UNIQUE,
    at      TEXT NOT NULL,
    machine TEXT NOT NULL,
    task    TEXT NOT NULL,
    kind    TEXT NOT NULL
            CHECK (kind IN ('created', 'title', 'area', 'fit', 'status', 'notes', 'day', 'check')),
    title   TEXT,
    area    TEXT,
    fit     TEXT,
    status  TEXT,
    notes   TEXT,
    -- O dia, como 2026-11-11. Num `day`, vazio quer dizer que a tarefa saiu
    -- da escalação; num `check`, é o dia da marcação.
    day     TEXT,
    -- Num `check`: 1 marcou, 0 desmarcou.
    checked INTEGER CHECK (checked IN (0, 1))
) STRICT;

INSERT INTO events_new (seq, id, at, machine, task, kind, title, area, fit, status, notes)
    SELECT seq, id, at, machine, task, kind, title, area, fit, status, notes
    FROM events
    ORDER BY seq;

DROP TABLE events;
ALTER TABLE events_new RENAME TO events;
CREATE INDEX events_by_task ON events (task);
