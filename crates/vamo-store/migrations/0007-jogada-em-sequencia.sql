-- 0007: a jogada em sequência.
--
-- Duas mudanças novas no log: `plan` (os passos da jogada, um por linha, ou
-- vazio pra tirar) e `progress` (o passo da vez, os itens marcados e a
-- espera, como `1,5,0`). A tabela é refeita, como na 0002 e na 0006, pra o
-- CHECK da coluna `kind` aceitar os tipos novos.

CREATE TABLE events_new (
    seq      INTEGER PRIMARY KEY,
    id       TEXT NOT NULL UNIQUE,
    at       TEXT NOT NULL,
    machine  TEXT NOT NULL,
    task     TEXT NOT NULL,
    kind     TEXT NOT NULL
             CHECK (kind IN ('created', 'title', 'area', 'fit', 'status', 'notes', 'day', 'check',
                             'repeat', 'plan', 'progress')),
    title    TEXT,
    area     TEXT,
    fit      TEXT,
    status   TEXT,
    notes    TEXT,
    day      TEXT,
    checked  INTEGER CHECK (checked IN (0, 1)),
    repeat   TEXT,
    plan     TEXT,
    progress TEXT
) STRICT;

INSERT INTO events_new (seq, id, at, machine, task, kind, title, area, fit, status, notes, day,
                        checked, repeat)
    SELECT seq, id, at, machine, task, kind, title, area, fit, status, notes, day, checked, repeat
    FROM events
    ORDER BY seq;

DROP TABLE events;
ALTER TABLE events_new RENAME TO events;
CREATE INDEX events_by_task ON events (task);
