//! Fila de downloads. O núcleo só guarda o estado; quem baixa é a camada de mídia, orquestrada
//! pela casca do app. `tracks.download_status` e `download_jobs.status` andam juntos.

use rusqlite::{OptionalExtension, Row, params};

use crate::db::{Database, new_id};
use crate::error::{Error, Result};
use crate::model::{DownloadJob, DownloadStatus, DownloadUpdate, DownloadedFile, Track};
use crate::text::clean_opt;

const JOB_SQL: &str = "SELECT id, track_id, status, progress, error FROM download_jobs";

fn job_from_row(r: &Row<'_>) -> rusqlite::Result<DownloadJob> {
    Ok(DownloadJob {
        id: r.get(0)?,
        track_id: r.get(1)?,
        status: r.get(2)?,
        progress: r.get(3)?,
        error: r.get(4)?,
    })
}

impl Database {
    // @req RF-10
    pub fn get_download(&self, job_id: &str) -> Result<DownloadJob> {
        self.conn
            .query_row(&format!("{JOB_SQL} WHERE id = ?1"), [job_id], job_from_row)
            .optional()?
            .ok_or_else(|| Error::not_found("download", job_id))
    }

    // @req RF-10
    /// Todos os jobs, do mais antigo ao mais novo.
    pub fn list_downloads(&self) -> Result<Vec<DownloadJob>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{JOB_SQL} ORDER BY created_at, id"))?;
        let rows = stmt.query_map([], job_from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // @req RF-10
    /// Próximo job `queued` (o mais antigo), para o orquestrador despachar.
    pub fn next_queued_download(&self) -> Result<Option<DownloadJob>> {
        Ok(self
            .conn
            .query_row(
                &format!("{JOB_SQL} WHERE status = 'queued' ORDER BY created_at, id LIMIT 1"),
                [],
                job_from_row,
            )
            .optional()?)
    }

    // @req RF-10
    /// Enfileira o download das faixas, uma vez cada, na ordem dada. Faixa já baixada é
    /// ignorada (não gera job); faixa com job `queued`/`downloading` devolve o job existente;
    /// faixa com job `error` é reenfileirada (o mesmo job volta a `queued`, progresso 0).
    pub fn enqueue_downloads(&self, track_ids: &[String]) -> Result<Vec<DownloadJob>> {
        self.in_tx(|| {
            let mut jobs: Vec<DownloadJob> = Vec::new();
            for tid in track_ids {
                if jobs.iter().any(|j| &j.track_id == tid) {
                    continue;
                }
                let track = self.get_track(tid)?;
                if track.download_status == DownloadStatus::Done {
                    continue;
                }
                let existing: Option<DownloadJob> = self
                    .conn
                    .query_row(&format!("{JOB_SQL} WHERE track_id = ?1"), [tid], job_from_row)
                    .optional()?;
                let now = self.now_ms();
                let job = match existing {
                    Some(j) if matches!(j.status, DownloadStatus::Queued | DownloadStatus::Downloading) => j,
                    Some(j) => {
                        self.conn.execute(
                            "UPDATE download_jobs SET status = 'queued', progress = 0, error = NULL,
                                    created_at = ?2, updated_at = ?2 WHERE id = ?1",
                            params![j.id, now],
                        )?;
                        self.set_track_status(tid, DownloadStatus::Queued)?;
                        self.get_download(&j.id)?
                    }
                    None => {
                        let id = new_id();
                        self.conn.execute(
                            "INSERT INTO download_jobs (id, track_id, status, progress, created_at, updated_at)
                             VALUES (?1, ?2, 'queued', 0, ?3, ?3)",
                            params![id, tid, now],
                        )?;
                        self.set_track_status(tid, DownloadStatus::Queued)?;
                        self.get_download(&id)?
                    }
                };
                jobs.push(job);
            }
            Ok(jobs)
        })
    }

    // @req RF-11
    /// Enfileira todas as faixas ainda não baixadas da playlist, na ordem dela. Devolve também
    /// os jobs que já estavam na fila.
    pub fn enqueue_playlist_downloads(&self, playlist_id: &str) -> Result<Vec<DownloadJob>> {
        self.in_tx(|| {
            self.get_playlist(playlist_id)?;
            let ids: Vec<String> = {
                let mut stmt = self.conn.prepare(
                    "SELECT t.id FROM playlist_tracks pt JOIN tracks t ON t.id = pt.track_id
                     WHERE pt.playlist_id = ?1 AND t.download_status <> 'done'
                     ORDER BY pt.position",
                )?;
                stmt.query_map([playlist_id], |r| r.get(0))?
                    .collect::<rusqlite::Result<_>>()?
            };
            self.enqueue_downloads(&ids)
        })
    }

    // @req RF-10
    /// Atualiza status/progresso/erro de um job e a faixa junto. `status` deve ser `queued`,
    /// `downloading` ou `error`; o sucesso passa por [`Database::mark_downloaded`]. Se o job foi
    /// cancelado (apagado), devolve `not_found`: sinal para o downloader abortar.
    pub fn update_download(&self, job_id: &str, update: &DownloadUpdate) -> Result<DownloadJob> {
        if !matches!(
            update.status,
            DownloadStatus::Queued | DownloadStatus::Downloading | DownloadStatus::Error
        ) {
            return Err(Error::invalid(
                "status de job deve ser queued, downloading ou error (done via mark_downloaded)",
            ));
        }
        if update.progress.is_some_and(|p| !p.is_finite()) {
            return Err(Error::invalid("progresso inválido"));
        }
        self.in_tx(|| {
            let job = self.get_download(job_id)?;
            let progress = update.progress.map_or(job.progress, |p| p.clamp(0.0, 1.0));
            let error = if update.status == DownloadStatus::Error {
                Some(
                    clean_opt(update.error.as_deref())
                        .unwrap_or_else(|| "erro desconhecido".into()),
                )
            } else {
                None
            };
            self.conn.execute(
                "UPDATE download_jobs SET status = ?2, progress = ?3, error = ?4, updated_at = ?5
                 WHERE id = ?1",
                params![job_id, update.status, progress, error, self.now_ms()],
            )?;
            self.set_track_status(&job.track_id, update.status)?;
            self.get_download(job_id)
        })
    }

    // @req RF-10 RF-05
    /// Conclui o download: grava arquivo, codec, bitrate (e capa/duração, se vierem), marca a
    /// faixa como `done` e fecha o job (progresso 1.0), se existir.
    pub fn mark_downloaded(&self, track_id: &str, file: &DownloadedFile) -> Result<Track> {
        let file_path = clean_opt(Some(&file.file_path))
            .ok_or_else(|| Error::invalid("o caminho do arquivo não pode ser vazio"))?;
        if file.duration_ms.is_some_and(|d| d < 0) {
            return Err(Error::invalid("a duração não pode ser negativa"));
        }
        self.in_tx(|| {
            self.require_track(track_id)?;
            self.conn.execute(
                "UPDATE tracks SET file_path = ?2, codec = ?3, bitrate = ?4,
                        cover_path = COALESCE(?5, cover_path),
                        duration_ms = CASE WHEN ?6 > 0 THEN ?6 ELSE duration_ms END,
                        download_status = 'done'
                 WHERE id = ?1",
                params![
                    track_id,
                    file_path,
                    clean_opt(file.codec.as_deref()),
                    file.bitrate,
                    clean_opt(file.cover_path.as_deref()),
                    file.duration_ms.unwrap_or(0),
                ],
            )?;
            self.conn.execute(
                "UPDATE download_jobs SET status = 'done', progress = 1, error = NULL,
                        updated_at = ?2 WHERE track_id = ?1",
                params![track_id, self.now_ms()],
            )?;
            self.get_track(track_id)
        })
    }

    // @req RF-10
    /// Cancela o job: apaga o registro e devolve a faixa para `none`. Job já concluído não
    /// pode ser cancelado (`conflict`).
    pub fn cancel_download(&self, job_id: &str) -> Result<()> {
        self.in_tx(|| {
            let job = self.get_download(job_id)?;
            if job.status == DownloadStatus::Done {
                return Err(Error::Conflict("o download já foi concluído".into()));
            }
            self.conn
                .execute("DELETE FROM download_jobs WHERE id = ?1", [job_id])?;
            self.set_track_status(&job.track_id, DownloadStatus::None)
        })
    }

    // @req RF-10
    /// Remove da lista os jobs concluídos. Devolve quantos.
    pub fn clear_finished_downloads(&self) -> Result<usize> {
        Ok(self
            .conn
            .execute("DELETE FROM download_jobs WHERE status = 'done'", [])?)
    }

    // @req RF-10
    /// Para chamar na abertura do app: jobs que estavam `downloading` quando o app fechou
    /// voltam para `queued`. Devolve quantos.
    pub fn requeue_interrupted_downloads(&self) -> Result<usize> {
        self.in_tx(|| {
            let n = self.conn.execute(
                "UPDATE download_jobs SET status = 'queued', progress = 0, updated_at = ?1
                 WHERE status = 'downloading'",
                [self.now_ms()],
            )?;
            self.conn.execute(
                "UPDATE tracks SET download_status = 'queued' WHERE download_status = 'downloading'",
                [],
            )?;
            Ok(n)
        })
    }

    fn set_track_status(&self, track_id: &str, status: DownloadStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE tracks SET download_status = ?2 WHERE id = ?1",
            params![track_id, status],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NewTrack, TrackQuery};
    use crate::testutil::*;

    fn yt(db: &Database, sid: &str) -> Track {
        db.upsert_track(&NewTrack::youtube(sid, sid, Some("Art".into()), 1000, None))
            .unwrap()
            .track
    }

    fn upd(status: DownloadStatus, progress: Option<f64>, error: Option<&str>) -> DownloadUpdate {
        DownloadUpdate {
            status,
            progress,
            error: error.map(Into::into),
        }
    }

    // @test RF-10
    #[test]
    fn enfileira_sem_duplicar_e_ignora_baixadas() {
        let db = db();
        let a = yt(&db, "a");
        let b = yt(&db, "b");
        let local = db
            .upsert_track(&NewTrack::local("/m/x.m4a", "X"))
            .unwrap()
            .track;

        let jobs = db
            .enqueue_downloads(&[a.id.clone(), b.id.clone(), a.id.clone(), local.id.clone()])
            .unwrap();
        assert_eq!(
            jobs.len(),
            2,
            "duplicada e local (já baixada) ficam de fora"
        );
        assert!(
            jobs.iter()
                .all(|j| j.status == DownloadStatus::Queued && j.progress == 0.0)
        );
        assert_eq!(
            db.get_track(&a.id).unwrap().download_status,
            DownloadStatus::Queued
        );

        let de_novo = db.enqueue_downloads(std::slice::from_ref(&a.id)).unwrap();
        assert_eq!(de_novo[0].id, jobs[0].id, "devolve o job existente");
        assert_eq!(db.list_downloads().unwrap().len(), 2);
        assert_eq!(
            db.enqueue_downloads(&["nope".into()]).unwrap_err().code(),
            "not_found"
        );
        assert_eq!(db.next_queued_download().unwrap().unwrap().id, jobs[0].id);
    }

    // @test RF-10
    #[test]
    fn ciclo_de_vida_progresso_erro_e_conclusao() {
        let db = db();
        let t = yt(&db, "a");
        let job = db
            .enqueue_downloads(std::slice::from_ref(&t.id))
            .unwrap()
            .remove(0);

        let j = db
            .update_download(&job.id, &upd(DownloadStatus::Downloading, Some(0.4), None))
            .unwrap();
        assert_eq!((j.status, j.progress), (DownloadStatus::Downloading, 0.4));
        assert_eq!(
            db.get_track(&t.id).unwrap().download_status,
            DownloadStatus::Downloading
        );
        // progresso fora da faixa é limitado; ausente mantém
        let j = db
            .update_download(&job.id, &upd(DownloadStatus::Downloading, Some(7.0), None))
            .unwrap();
        assert_eq!(j.progress, 1.0);
        let j = db
            .update_download(&job.id, &upd(DownloadStatus::Downloading, None, None))
            .unwrap();
        assert_eq!(j.progress, 1.0);

        let j = db
            .update_download(&job.id, &upd(DownloadStatus::Error, None, Some("sem rede")))
            .unwrap();
        assert_eq!(
            (j.status, j.error.as_deref()),
            (DownloadStatus::Error, Some("sem rede"))
        );
        assert_eq!(
            db.get_track(&t.id).unwrap().download_status,
            DownloadStatus::Error
        );

        // tentar de novo reaproveita o mesmo job
        let retry = db
            .enqueue_downloads(std::slice::from_ref(&t.id))
            .unwrap()
            .remove(0);
        assert_eq!(retry.id, job.id);
        assert_eq!(
            (retry.status, retry.progress, retry.error),
            (DownloadStatus::Queued, 0.0, None)
        );

        let done = db
            .mark_downloaded(
                &t.id,
                &DownloadedFile {
                    file_path: "/audio/a.m4a".into(),
                    codec: Some("aac".into()),
                    bitrate: Some(128_000),
                    cover_path: Some("/covers/a.jpg".into()),
                    duration_ms: Some(215_000),
                },
            )
            .unwrap();
        assert_eq!(done.download_status, DownloadStatus::Done);
        assert_eq!(done.file_path.as_deref(), Some("/audio/a.m4a"));
        assert_eq!(
            (done.codec.as_deref(), done.bitrate),
            (Some("aac"), Some(128_000))
        );
        assert_eq!(done.cover_path.as_deref(), Some("/covers/a.jpg"));
        assert_eq!(done.duration_ms, 215_000);
        let j = db.get_download(&job.id).unwrap();
        assert_eq!((j.status, j.progress), (DownloadStatus::Done, 1.0));

        // concluída: não reenfileira, não cancela, mas pode ser limpa da lista
        assert!(
            db.enqueue_downloads(std::slice::from_ref(&t.id))
                .unwrap()
                .is_empty()
        );
        assert_eq!(db.cancel_download(&job.id).unwrap_err().code(), "conflict");
        assert_eq!(db.clear_finished_downloads().unwrap(), 1);
        assert!(db.list_downloads().unwrap().is_empty());
    }

    // @test RF-10
    #[test]
    fn update_valida_status() {
        let db = db();
        let t = yt(&db, "a");
        let job = db.enqueue_downloads(&[t.id]).unwrap().remove(0);
        for s in [DownloadStatus::Done, DownloadStatus::None] {
            assert_eq!(
                db.update_download(&job.id, &upd(s, None, None))
                    .unwrap_err()
                    .code(),
                "invalid_input"
            );
        }
        let nan = upd(DownloadStatus::Downloading, Some(f64::NAN), None);
        assert_eq!(
            db.update_download(&job.id, &nan).unwrap_err().code(),
            "invalid_input"
        );
    }

    // @test RF-10
    #[test]
    fn cancelar_apaga_o_job_e_libera_a_faixa() {
        let db = db();
        let t = yt(&db, "a");
        let job = db
            .enqueue_downloads(std::slice::from_ref(&t.id))
            .unwrap()
            .remove(0);
        db.update_download(&job.id, &upd(DownloadStatus::Downloading, Some(0.2), None))
            .unwrap();
        db.cancel_download(&job.id).unwrap();
        assert_eq!(
            db.get_track(&t.id).unwrap().download_status,
            DownloadStatus::None
        );
        assert!(db.list_downloads().unwrap().is_empty());
        assert_eq!(db.cancel_download(&job.id).unwrap_err().code(), "not_found");
        // o downloader descobre o cancelamento ao tentar atualizar
        let r = db.update_download(&job.id, &upd(DownloadStatus::Downloading, Some(0.5), None));
        assert_eq!(r.unwrap_err().code(), "not_found");
    }

    // @test RF-11
    #[test]
    fn enfileira_playlist_so_as_nao_baixadas_na_ordem() {
        let db = db();
        let a = yt(&db, "a");
        let b = yt(&db, "b");
        let c = yt(&db, "c");
        let local = db
            .upsert_track(&NewTrack::local("/m/x.m4a", "X"))
            .unwrap()
            .track;
        let pl = db.create_playlist("P", None).unwrap();
        db.add_tracks_to_playlist(
            &pl.id,
            &[c.id.clone(), local.id.clone(), a.id.clone(), b.id.clone()],
        )
        .unwrap();
        db.mark_downloaded(
            &b.id,
            &DownloadedFile {
                file_path: "/m/b.m4a".into(),
                codec: None,
                bitrate: None,
                cover_path: None,
                duration_ms: None,
            },
        )
        .unwrap();

        let jobs = db.enqueue_playlist_downloads(&pl.id).unwrap();
        let tracks: Vec<&str> = jobs.iter().map(|j| j.track_id.as_str()).collect();
        assert_eq!(
            tracks,
            [c.id.as_str(), a.id.as_str()],
            "ordem da playlist, sem local nem baixada"
        );
        // de novo: devolve os mesmos jobs, sem criar outros
        let de_novo = db.enqueue_playlist_downloads(&pl.id).unwrap();
        assert_eq!(de_novo, jobs);
        assert_eq!(db.list_downloads().unwrap().len(), 2);
        assert_eq!(
            db.enqueue_playlist_downloads("nope").unwrap_err().code(),
            "not_found"
        );
    }

    // @test RF-10
    #[test]
    fn requeue_apos_queda_e_cascata_ao_apagar_faixa() {
        let db = db();
        let a = yt(&db, "a");
        let b = yt(&db, "b");
        let jobs = db.enqueue_downloads(&[a.id.clone(), b.id.clone()]).unwrap();
        db.update_download(
            &jobs[0].id,
            &upd(DownloadStatus::Downloading, Some(0.9), None),
        )
        .unwrap();
        assert_eq!(db.requeue_interrupted_downloads().unwrap(), 1);
        let j = db.get_download(&jobs[0].id).unwrap();
        assert_eq!((j.status, j.progress), (DownloadStatus::Queued, 0.0));
        assert_eq!(
            db.get_track(&a.id).unwrap().download_status,
            DownloadStatus::Queued
        );

        db.delete_track(&a.id).unwrap();
        assert_eq!(db.list_downloads().unwrap().len(), 1);
        let _ = db.list_tracks(&TrackQuery::default()).unwrap();
    }
}
