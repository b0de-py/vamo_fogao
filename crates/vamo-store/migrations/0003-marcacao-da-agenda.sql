-- 0003: a marcação das atividades fixas da agenda.
--
-- Um compromisso recorrente da agenda vira item de marcar na tela Hoje sem
-- virar tarefa: ele não tem ID de tarefa, então não entra no log de
-- eventos. Cada linha aqui é "o evento `uid` foi marcado no dia `day`";
-- desmarcar apaga a linha.

CREATE TABLE agenda_checks (
    -- O dia, como 2026-11-11.
    day TEXT NOT NULL,
    -- O UID do evento na agenda.
    uid TEXT NOT NULL,
    PRIMARY KEY (day, uid)
) STRICT;
