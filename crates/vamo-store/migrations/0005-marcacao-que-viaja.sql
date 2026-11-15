-- 0005: a marcação da agenda passa a viajar entre as máquinas.
--
-- Pra mais recente ganhar na sincronia (ADR 0013), cada marcação guarda
-- quando foi feita, e desmarcar deixa de apagar a linha: vira `done = 0`,
-- pra outra máquina saber que a desmarcação é mais nova que a marcação
-- dela. As linhas de antes ficam marcadas, com o instante zero, e perdem
-- de qualquer mudança feita depois.

ALTER TABLE agenda_checks ADD COLUMN done INTEGER NOT NULL DEFAULT 1;
ALTER TABLE agenda_checks ADD COLUMN at TEXT NOT NULL DEFAULT '1970-01-01T00:00:00Z';
