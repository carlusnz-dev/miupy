//! Perfis do equalizador e persistência dos perfis personalizados.
use crate::db::new_id;
use crate::{Database, Error, Result};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

pub const BUILTINS: [(&str, &str, [f32; 8]); 8] = [
    ("flat", "Plano", [0., 0., 0., 0., 0., 0., 0., 0.]),
    ("bass", "Graves", [6., 5., 3., 1., 0., 0., 0., 0.]),
    ("treble", "Agudos", [0., 0., 0., 0., 1., 3., 5., 6.]),
    ("vocal", "Vocal", [-2., -1., 0., 2., 4., 4., 2., 0.]),
    ("rock", "Rock", [4., 3., -1., -2., -1., 2., 4., 5.]),
    ("pop", "Pop", [-1., 1., 3., 4., 3., 0., -1., -1.]),
    (
        "electronic",
        "Eletrônica",
        [5., 4., 1., 0., -1., 2., 3., 4.],
    ),
    ("acoustic", "Acústico", [3., 2., 1., 1., 2., 2., 3., 2.]),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqPreset {
    pub id: String,
    pub name: String,
    pub builtin: bool,
    pub gains_db: Vec<f32>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqPresetInput {
    pub id: Option<String>,
    pub name: String,
    pub gains_db: Vec<f32>,
}

impl Database {
    // @req RF-29
    pub fn list_eq_presets(&self) -> Result<Vec<EqPreset>> {
        let mut out: Vec<_> = BUILTINS
            .iter()
            .map(|(id, name, gains)| EqPreset {
                id: (*id).into(),
                name: (*name).into(),
                builtin: true,
                gains_db: gains.to_vec(),
            })
            .collect();
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, gains_db FROM eq_presets ORDER BY created_at, rowid")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (id, name, json) = row?;
            let gains_db =
                serde_json::from_str(&json).map_err(|e| Error::Internal(e.to_string()))?;
            out.push(EqPreset {
                id,
                name,
                builtin: false,
                gains_db,
            });
        }
        Ok(out)
    }
    // @req RF-29
    pub fn get_eq_preset(&self, id: &str) -> Result<EqPreset> {
        if let Some((_, name, gains)) = BUILTINS.iter().find(|(key, _, _)| *key == id) {
            return Ok(EqPreset {
                id: id.into(),
                name: (*name).into(),
                builtin: true,
                gains_db: gains.to_vec(),
            });
        }
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT name,gains_db FROM eq_presets WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (name, json) = row.ok_or_else(|| Error::not_found("perfil", id))?;
        let gains_db = serde_json::from_str(&json).map_err(|e| Error::Internal(e.to_string()))?;
        Ok(EqPreset {
            id: id.into(),
            name,
            builtin: false,
            gains_db,
        })
    }
    // @req RF-29
    pub fn save_eq_preset(&self, input: &EqPresetInput) -> Result<EqPreset> {
        let name = input.name.trim();
        if name.is_empty()
            || name.chars().count() > 30
            || input.gains_db.len() != 8
            || input
                .gains_db
                .iter()
                .any(|v| !v.is_finite() || !(-12.0..=12.0).contains(v))
        {
            return Err(Error::invalid("nome ou ganhos do equalizador inválidos"));
        }
        if let Some(id) = &input.id {
            if BUILTINS.iter().any(|(key, _, _)| *key == id) {
                return Err(Error::invalid("perfil pronto não pode ser editado"));
            }
            self.get_eq_preset(id)?;
        } else {
            let count: i64 = self
                .conn
                .query_row("SELECT count(*) FROM eq_presets", [], |r| r.get(0))?;
            if count >= 3 {
                return Err(Error::Conflict(
                    "limite de três perfis personalizados".into(),
                ));
            }
        }
        if BUILTINS
            .iter()
            .any(|(_, built, _)| built.to_lowercase() == name.to_lowercase())
        {
            return Err(Error::Conflict("nome de perfil já existe".into()));
        }
        if self.list_eq_presets()?.iter().any(|p| {
            p.id != input.id.as_deref().unwrap_or("")
                && p.name.to_lowercase() == name.to_lowercase()
        }) {
            return Err(Error::Conflict("nome de perfil já existe".into()));
        }
        let id = input.id.clone().unwrap_or_else(new_id);
        let json =
            serde_json::to_string(&input.gains_db).map_err(|e| Error::Internal(e.to_string()))?;
        if input.id.is_some() {
            self.conn.execute(
                "UPDATE eq_presets SET name=?2,gains_db=?3 WHERE id=?1",
                params![id, name, json],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO eq_presets(id,name,gains_db,created_at) VALUES (?1,?2,?3,?4)",
                params![id, name, json, self.now_ms()],
            )?;
        }
        self.get_eq_preset(&id)
    }
    // @req RF-29
    pub fn delete_eq_preset(&self, id: &str) -> Result<()> {
        if BUILTINS.iter().any(|(key, _, _)| *key == id) {
            return Err(Error::invalid("perfil pronto não pode ser apagado"));
        }
        self.in_tx(|| {
            if self
                .conn
                .execute("DELETE FROM eq_presets WHERE id=?1", [id])?
                == 0
            {
                return Err(Error::not_found("perfil", id));
            }
            self.conn.execute(
                "UPDATE settings SET eq_preset_id='flat' WHERE eq_preset_id=?1",
                [id],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // @test RF-29
    #[test]
    fn crud_and_errors() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.list_eq_presets().unwrap().len(), 8);
        assert_eq!(db.get_settings().unwrap().eq_preset_id, "flat");
        for n in 0..3 {
            db.save_eq_preset(&EqPresetInput {
                id: None,
                name: format!("Meu {n}"),
                gains_db: vec![0.; 8],
            })
            .unwrap();
        }
        assert_eq!(
            db.save_eq_preset(&EqPresetInput {
                id: None,
                name: "Novo".into(),
                gains_db: vec![0.; 8]
            })
            .unwrap_err()
            .code(),
            "conflict"
        );
        let id = db.list_eq_presets().unwrap()[8].id.clone();
        db.update_settings(&crate::SettingsPatch {
            eq_preset_id: Some(id.clone()),
            ..Default::default()
        })
        .unwrap();
        db.delete_eq_preset(&id).unwrap();
        assert_eq!(db.get_settings().unwrap().eq_preset_id, "flat");
        assert_eq!(db.delete_eq_preset(&id).unwrap_err().code(), "not_found");
        assert_eq!(
            db.delete_eq_preset("bass").unwrap_err().code(),
            "invalid_input"
        );
        assert_eq!(
            db.update_settings(&crate::SettingsPatch {
                eq_preset_id: Some("ausente".into()),
                ..Default::default()
            })
            .unwrap_err()
            .code(),
            "not_found"
        );
        assert_eq!(
            db.save_eq_preset(&EqPresetInput {
                id: Some("flat".into()),
                name: "X".into(),
                gains_db: vec![0.; 8]
            })
            .unwrap_err()
            .code(),
            "invalid_input"
        );
        assert_eq!(
            db.save_eq_preset(&EqPresetInput {
                id: None,
                name: "".into(),
                gains_db: vec![0.; 8]
            })
            .unwrap_err()
            .code(),
            "invalid_input"
        );
        assert_eq!(
            db.save_eq_preset(&EqPresetInput {
                id: None,
                name: "X".into(),
                gains_db: vec![13.; 8]
            })
            .unwrap_err()
            .code(),
            "invalid_input"
        );
        assert_eq!(
            db.save_eq_preset(&EqPresetInput {
                id: None,
                name: "X".into(),
                gains_db: vec![0.; 7]
            })
            .unwrap_err()
            .code(),
            "invalid_input"
        );
    }

    fn input(id: Option<&str>, name: &str, gain: f32) -> EqPresetInput {
        EqPresetInput {
            id: id.map(Into::into),
            name: name.into(),
            gains_db: vec![gain; 8],
        }
    }
    // @test RF-29
    #[test]
    fn duplicate_name_ignoring_case_is_conflict() {
        let db = Database::open_in_memory().unwrap();
        db.save_eq_preset(&input(None, "Meu Perfil", 1.)).unwrap();
        for name in [
            "meu perfil",
            "MEU PERFIL",
            "  Meu Perfil  ",
            "plano",
            "ROCK",
        ] {
            assert_eq!(
                db.save_eq_preset(&input(None, name, 1.))
                    .unwrap_err()
                    .code(),
                "conflict",
                "{name}"
            );
        }
    }
    // @test RF-29
    #[test]
    fn name_length_limit_is_30_characters() {
        let db = Database::open_in_memory().unwrap();
        let ok = "é".repeat(30);
        assert_eq!(db.save_eq_preset(&input(None, &ok, 0.)).unwrap().name, ok);
        let too_long = "a".repeat(31);
        assert_eq!(
            db.save_eq_preset(&input(None, &too_long, 0.))
                .unwrap_err()
                .code(),
            "invalid_input"
        );
    }
    // @test RF-29
    #[test]
    fn edit_keeps_id_and_changes_name_and_gains() {
        let db = Database::open_in_memory().unwrap();
        let created = db.save_eq_preset(&input(None, "Antes", 1.)).unwrap();
        let edited = db
            .save_eq_preset(&input(Some(&created.id), "Depois", -3.))
            .unwrap();
        assert_eq!(edited.id, created.id);
        assert_eq!(edited.name, "Depois");
        assert_eq!(edited.gains_db, vec![-3.; 8]);
        assert!(!edited.builtin);
        let list = db.list_eq_presets().unwrap();
        assert_eq!(list.len(), 9);
        assert_eq!(list[8], edited);
        // Reaproveitar o próprio nome (outra caixa) ao editar não é conflito.
        db.save_eq_preset(&input(Some(&created.id), "DEPOIS", 2.))
            .unwrap();
        assert_eq!(
            db.save_eq_preset(&input(Some("inexistente"), "X", 0.))
                .unwrap_err()
                .code(),
            "not_found"
        );
    }
    // @test RF-29
    #[test]
    fn list_has_builtins_first_then_custom_in_creation_order() {
        let db = Database::open_in_memory().unwrap();
        let a = db.save_eq_preset(&input(None, "C", 1.)).unwrap();
        let b = db.save_eq_preset(&input(None, "A", 2.)).unwrap();
        let c = db.save_eq_preset(&input(None, "B", 3.)).unwrap();
        // Editar não muda a posição.
        db.save_eq_preset(&input(Some(&a.id), "C2", 4.)).unwrap();
        let list = db.list_eq_presets().unwrap();
        let ids: Vec<&str> = list.iter().map(|p| p.id.as_str()).collect();
        let builtin_ids: Vec<&str> = BUILTINS.iter().map(|(id, _, _)| *id).collect();
        assert_eq!(&ids[..8], &builtin_ids[..]);
        assert!(list[..8].iter().all(|p| p.builtin));
        assert_eq!(&ids[8..], [a.id.as_str(), b.id.as_str(), c.id.as_str()]);
        assert!(list[8..].iter().all(|p| !p.builtin));
    }
    // @test RF-29
    #[test]
    fn at_most_three_custom_and_slot_frees_after_delete() {
        let db = Database::open_in_memory().unwrap();
        let mut ids = vec![];
        for n in 0..3 {
            ids.push(
                db.save_eq_preset(&input(None, &format!("P{n}"), 0.))
                    .unwrap()
                    .id,
            );
        }
        assert_eq!(
            db.save_eq_preset(&input(None, "P3", 0.))
                .unwrap_err()
                .code(),
            "conflict"
        );
        // Editar com 3 existentes continua permitido.
        db.save_eq_preset(&input(Some(&ids[0]), "P0b", 1.)).unwrap();
        db.delete_eq_preset(&ids[1]).unwrap();
        db.save_eq_preset(&input(None, "P3", 0.)).unwrap();
        assert_eq!(db.list_eq_presets().unwrap().len(), 11);
    }
    // @test RF-29
    #[test]
    fn deleting_active_custom_falls_back_to_flat_but_keeps_other_selection() {
        let db = Database::open_in_memory().unwrap();
        let a = db.save_eq_preset(&input(None, "A", 1.)).unwrap();
        let b = db.save_eq_preset(&input(None, "B", 2.)).unwrap();
        db.update_settings(&crate::SettingsPatch {
            eq_preset_id: Some(b.id.clone()),
            ..Default::default()
        })
        .unwrap();
        db.delete_eq_preset(&a.id).unwrap();
        assert_eq!(db.get_settings().unwrap().eq_preset_id, b.id);
        db.delete_eq_preset(&b.id).unwrap();
        assert_eq!(db.get_settings().unwrap().eq_preset_id, "flat");
    }
}
