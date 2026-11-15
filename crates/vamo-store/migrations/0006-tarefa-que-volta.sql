-- 0006: o intervalo da tarefa que volta.
--
-- Uma mudança nova no log: `repeat` (a tarefa passou a voltar de tantos em
-- tantos meses ou semanas depois do gol, ou deixou de voltar). O SQLite não
-- deixa trocar o CHECK de uma coluna, então a tabela é refeita, como na 0002,
-- e os eventos antigos são copiados na mesma ordem.

CREATE TABLE events_new (
    seq     INTEGER PRIMARY KEY,
    id      TEXT NOT NULL UNIQUE,
    at      TEXT NOT NULL,
    machine TEXT NOT NULL,
    task    TEXT NOT NULL,
    kind    TEXT NOT NULL
            CHECK (kind IN ('created', 'title', 'area', 'fit', 'status', 'notes', 'day', 'check', 'repeat')),
    title   TEXT,
    area    TEXT,
    fit     TEXT,
    status  TEXT,
    notes   TEXT,
    day     TEXT,
    checked INTEGER CHECK (checked IN (0, 1)),
    -- Num `repeat`: o intervalo como `1m` ou `2s`. Vazio quer dizer que a
    -- tarefa deixou de voltar.
    repeat  TEXT
) STRICT;

INSERT INTO events_new (seq, id, at, machine, task, kind, title, area, fit, status, notes, day, checked)
    SELECT seq, id, at, machine, task, kind, title, area, fit, status, notes, day, checked
    FROM events
    ORDER BY seq;

DROP TABLE events;
ALTER TABLE events_new RENAME TO events;
CREATE INDEX events_by_task ON events (task);
