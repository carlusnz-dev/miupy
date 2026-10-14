//! Registro de atividade (RF-13): eventos de reprodução e log genérico de ações.
//!
//! Fluxo de um evento: [`Database::start_play`] ao iniciar a faixa, [`Database::add_listened`]
//! a cada tick enquanto toca (pausa não conta: quem chama só soma o tempo tocado) e
//! [`Database::finish_play`] ao terminar, pular ou parar.

use rusqlite::{OptionalExtension, Row, params};

use crate::db::{Database, new_id};
use crate::error::{Error, Result};
use crate::model::{PlayEnd, PlayEvent, UserAction};

const EVENT_SQL: &str = "SELECT id, track_id, playlist_id, started_at, ended_at, listened_ms,
        completed, skipped FROM play_events";

fn event_from_row(r: &Row<'_>) -> rusqlite::Result<PlayEvent> {
    Ok(PlayEvent {
        id: r.get(0)?,
        track_id: r.get(1)?,
        playlist_id: r.get(2)?,
        started_at: r.get(3)?,
        ended_at: r.get(4)?,
        listened_ms: r.get(5)?,
        completed: r.get(6)?,
        skipped: r.get(7)?,
    })
}

impl Database {
    // @req RF-13
    /// Abre um evento de reprodução agora. `playlist_id` é o contexto (de que playlist a
    /// faixa está tocando), opcional. Devolve o id do evento.
    pub fn start_play(&self, track_id: &str, playlist_id: Option<&str>) -> Result<String> {
        self.start_play_at(track_id, playlist_id, self.now_ms())
    }

    // @req RF-13
    /// Como [`Database::start_play`], com o instante de início explícito (importação, testes).
    pub fn start_play_at(
        &self,
        track_id: &str,
        playlist_id: Option<&str>,
        started_at: i64,
    ) -> Result<String> {
        self.in_tx(|| {
            self.require_track(track_id)?;
            if let Some(pid) = playlist_id {
                self.get_playlist(pid)?;
            }
            let id = new_id();
            self.conn.execute(
                "INSERT INTO play_events (id, track_id, playlist_id, started_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, track_id, playlist_id, started_at],
            )?;
            Ok(id)
        })
    }

    // @req RF-13
    /// Soma `delta_ms` (>= 0) ao tempo realmente ouvido do evento. Atômico no banco.
    pub fn add_listened(&self, event_id: &str, delta_ms: i64) -> Result<()> {
        if delta_ms < 0 {
            return Err(Error::invalid("o tempo ouvido não pode ser negativo"));
        }
        let n = self.conn.execute(
            "UPDATE play_events SET listened_ms = listened_ms + ?2 WHERE id = ?1",
            params![event_id, delta_ms],
        )?;
        if n == 0 {
            Err(Error::not_found("evento de reprodução", event_id))
        } else {
            Ok(())
        }
    }

    // @req RF-13
    /// Fecha o evento. `Completed` marca `completed`; `Skipped` marca `skipped`; `Stopped` não
    /// marca nenhum. Fechar de novo sobrescreve o desfecho (idempotente).
    pub fn finish_play(&self, event_id: &str, end: PlayEnd) -> Result<PlayEvent> {
        let (completed, skipped) = match end {
            PlayEnd::Completed => (1, 0),
            PlayEnd::Skipped => (0, 1),
            PlayEnd::Stopped => (0, 0),
        };
        self.in_tx(|| {
            let n = self.conn.execute(
                "UPDATE play_events SET ended_at = ?2, completed = ?3, skipped = ?4 WHERE id = ?1",
                params![event_id, self.now_ms(), completed, skipped],
            )?;
            if n == 0 {
                return Err(Error::not_found("evento de reprodução", event_id));
            }
            self.get_play_event(event_id)
        })
    }

    // @req RF-13
    pub fn get_play_event(&self, event_id: &str) -> Result<PlayEvent> {
        self.conn
            .query_row(
                &format!("{EVENT_SQL} WHERE id = ?1"),
                [event_id],
                event_from_row,
            )
            .optional()?
            .ok_or_else(|| Error::not_found("evento de reprodução", event_id))
    }

    // @req RF-13
    /// Para chamar na abertura do app: fecha (como `Stopped`) eventos que ficaram abertos
    /// por queda do processo, mantendo o tempo já somado. Devolve quantos.
    pub fn close_dangling_play_events(&self) -> Result<usize> {
        Ok(self.conn.execute(
            "UPDATE play_events SET ended_at = started_at + listened_ms WHERE ended_at IS NULL",
            [],
        )?)
    }

    // @req RF-13
    /// Registra uma ação do usuário no log genérico (ex.: `track_added`, `playlist_created`).
    /// As operações deste crate já registram as suas; use isto para ações novas.
    pub fn log_action(
        &self,
        kind: &str,
        entity_type: Option<&str>,
        entity_id: Option<&str>,
        payload: Option<&serde_json::Value>,
    ) -> Result<UserAction> {
        let kind = kind.trim();
        if kind.is_empty() {
            return Err(Error::invalid("o tipo da ação não pode ser vazio"));
        }
        let id = self.record_action(kind, entity_type, entity_id, payload)?;
        self.get_action(&id)
    }

    pub(crate) fn record_action(
        &self,
        kind: &str,
        entity_type: Option<&str>,
        entity_id: Option<&str>,
        payload: Option<&serde_json::Value>,
    ) -> Result<String> {
        let id = new_id();
        let payload = payload.map(serde_json::Value::to_string);
        self.conn.execute(
            "INSERT INTO user_actions (id, kind, entity_type, entity_id, payload, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, kind, entity_type, entity_id, payload, self.now_ms()],
        )?;
        Ok(id)
    }

    fn get_action(&self, id: &str) -> Result<UserAction> {
        Ok(self.conn.query_row(
            "SELECT id, kind, entity_type, entity_id, payload, created_at
             FROM user_actions WHERE id = ?1",
            [id],
            action_from_row,
        )?)
    }

    // @req RF-13
    /// Ações mais recentes primeiro, opcionalmente só de um `kind`.
    pub fn list_actions(&self, kind: Option<&str>, limit: u32) -> Result<Vec<UserAction>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, entity_type, entity_id, payload, created_at FROM user_actions
             WHERE (?1 IS NULL OR kind = ?1)
             ORDER BY created_at DESC, id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![kind, limit], action_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

fn action_from_row(r: &Row<'_>) -> rusqlite::Result<UserAction> {
    let payload: Option<String> = r.get(4)?;
    Ok(UserAction {
        id: r.get(0)?,
        kind: r.get(1)?,
        entity_type: r.get(2)?,
        entity_id: r.get(3)?,
        payload: payload.and_then(|p| serde_json::from_str(&p).ok()),
        created_at: r.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering};

    use super::*;
    use crate::testutil::*;

    // @test RF-13
    #[test]
    fn evento_de_reproducao_abre_soma_e_fecha() {
        let mut db = db();
        let relogio = Arc::new(AtomicI64::new(1_000));
        let r = relogio.clone();
        db.set_clock(move || r.load(Ordering::SeqCst));

        let t = add_track(&db, "A", "X", 200_000);
        let pl = db.create_playlist("P", None).unwrap();
        let ev = db.start_play(&t.id, Some(&pl.id)).unwrap();
        let e = db.get_play_event(&ev).unwrap();
        assert_eq!((e.started_at, e.listened_ms, e.ended_at), (1_000, 0, None));
        assert_eq!(e.playlist_id.as_deref(), Some(pl.id.as_str()));

        db.add_listened(&ev, 10_000).unwrap();
        db.add_listened(&ev, 5_500).unwrap();
        relogio.store(40_000, Ordering::SeqCst);
        let e = db.finish_play(&ev, PlayEnd::Skipped).unwrap();
        assert_eq!(e.listened_ms, 15_500);
        assert_eq!(e.ended_at, Some(40_000));
        assert!(e.skipped && !e.completed);

        let e = db.finish_play(&ev, PlayEnd::Completed).unwrap();
        assert!(e.completed && !e.skipped);
        let e = db.finish_play(&ev, PlayEnd::Stopped).unwrap();
        assert!(!e.completed && !e.skipped);
    }

    // @test RF-13
    #[test]
    fn valida_entradas_do_evento() {
        let db = db();
        let t = add_track(&db, "A", "X", 1000);
        assert_eq!(db.start_play("nope", None).unwrap_err().code(), "not_found");
        assert_eq!(
            db.start_play(&t.id, Some("nope")).unwrap_err().code(),
            "not_found"
        );
        let ev = db.start_play(&t.id, None).unwrap();
        assert_eq!(
            db.add_listened(&ev, -1).unwrap_err().code(),
            "invalid_input"
        );
        assert_eq!(db.add_listened("nope", 1).unwrap_err().code(), "not_found");
        assert_eq!(
            db.finish_play("nope", PlayEnd::Completed)
                .unwrap_err()
                .code(),
            "not_found"
        );
    }

    // @test RF-13
    #[test]
    fn fecha_eventos_pendurados_mantendo_o_tempo() {
        let db = db();
        let t = add_track(&db, "A", "X", 1000);
        let aberto = db.start_play_at(&t.id, None, 100).unwrap();
        db.add_listened(&aberto, 50).unwrap();
        let fechado = db.start_play_at(&t.id, None, 500).unwrap();
        db.finish_play(&fechado, PlayEnd::Completed).unwrap();
        assert_eq!(db.close_dangling_play_events().unwrap(), 1);
        let e = db.get_play_event(&aberto).unwrap();
        assert_eq!((e.ended_at, e.listened_ms), (Some(150), 50));
    }

    // @test RF-13
    #[test]
    fn log_de_acoes_generico_e_automatico() {
        let db = db();
        let t = add_track(&db, "A", "X", 1000); // registra track_added
        db.create_playlist("P", None).unwrap(); // registra playlist_created
        let a = db
            .log_action(
                "custom",
                Some("track"),
                Some(&t.id),
                Some(&serde_json::json!({"k": 1})),
            )
            .unwrap();
        assert_eq!(a.payload.unwrap()["k"], 1);
        assert_eq!(
            db.log_action("  ", None, None, None).unwrap_err().code(),
            "invalid_input"
        );

        let todas = db.list_actions(None, 10).unwrap();
        let kinds: Vec<&str> = todas.iter().map(|a| a.kind.as_str()).collect();
        assert_eq!(kinds, ["custom", "playlist_created", "track_added"]);
        assert_eq!(db.list_actions(Some("track_added"), 10).unwrap().len(), 1);
        assert_eq!(db.list_actions(None, 1).unwrap().len(), 1);

        // o log sobrevive à remoção da faixa
        db.delete_track(&t.id).unwrap();
        assert_eq!(db.list_actions(Some("track_added"), 10).unwrap().len(), 1);
        assert_eq!(db.list_actions(Some("track_removed"), 10).unwrap().len(), 1);
    }
}
