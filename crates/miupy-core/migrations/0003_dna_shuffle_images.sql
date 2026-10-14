-- @req RF-24 RF-27 RNF-09
-- Colunas aditivas: preservam artistas, faixas e configurações da versão 2.
ALTER TABLE artists ADD COLUMN image_credit TEXT;
ALTER TABLE settings ADD COLUMN shuffle_mode TEXT NOT NULL DEFAULT 'off'
    CHECK (shuffle_mode IN ('off', 'random', 'smart'));
-- Modo de aceleração de hardware escolhido nas configurações; vale na próxima abertura.
ALTER TABLE settings ADD COLUMN gpu_mode TEXT NOT NULL DEFAULT 'auto'
    CHECK (gpu_mode IN ('auto', 'on', 'off'));
