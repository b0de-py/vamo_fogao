-- 0004: as súmulas fechadas.
--
-- A súmula se calcula do log, mas fica guardada como saiu no apito final:
-- é ela que vai pro backup e pro MCP, e os números dos hábitos vêm de fora
-- do log. Uma linha por dia; fechar o mesmo dia de novo troca a linha.

CREATE TABLE summaries (
    -- O dia, como 2026-11-11.
    day TEXT PRIMARY KEY,
    -- Quando a súmula foi fechada, em RFC 3339 e UTC.
    closed_at TEXT NOT NULL,
    -- A súmula no formato de exportação (JSON).
    content TEXT NOT NULL
) STRICT;
