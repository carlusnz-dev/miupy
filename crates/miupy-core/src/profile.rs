//! Perfil (RF-16) e configurações (RF-17), ambos de linha única.

use rusqlite::params;

use crate::db::Database;
use crate::error::{Error, Result};
use crate::model::{Profile, Settings, SettingsPatch};
use crate::text::clean_opt;

const MAX_NAME_CHARS: usize = 100;

/// E-mail tolerante: uma `@`, parte local e domínio não vazios, domínio com ponto entre
/// caracteres, sem espaços. Não tenta cobrir a RFC 5322.
fn valid_email(s: &str) -> bool {
    if s.chars().any(char::is_whitespace) {
        return false;
    }
    let mut parts = s.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains("..")
}

/// Telefone tolerante: dígitos com `+`, espaços, `-`, `.`, `(` e `)`; 8 a 15 dígitos.
fn valid_phone(s: &str) -> bool {
    let digits = s.chars().filter(char::is_ascii_digit).count();
    let only_allowed = s
        .chars()
        .all(|c| c.is_ascii_digit() || " +-.()".contains(c));
    only_allowed && (8..=15).contains(&digits)
}

/// Nome de usuário: 1 a 32 caracteres entre letras, dígitos, `_`, `.` e `-`.
fn valid_username(s: &str) -> bool {
    (1..=32).contains(&s.chars().count())
        && s.chars().all(|c| c.is_alphanumeric() || "_.-".contains(c))
}

impl Database {
    // @req RF-16
    pub fn get_profile(&self) -> Result<Profile> {
        Ok(self.conn.query_row(
            "SELECT display_name, username, email, phone, avatar_path FROM profile WHERE id = 1",
            [],
            |r| {
                Ok(Profile {
                    display_name: r.get(0)?,
                    username: r.get(1)?,
                    email: r.get(2)?,
                    phone: r.get(3)?,
                    avatar_path: r.get(4)?,
                })
            },
        )?)
    }

    // @req RF-16
    /// Substitui o perfil inteiro (como em `profile_update`). Campos são aparados. E-mail,
    /// telefone e nome de usuário podem ficar vazios; se preenchidos, o formato é conferido.
    pub fn update_profile(&self, profile: &Profile) -> Result<Profile> {
        let display_name = profile.display_name.trim();
        let username = profile.username.trim();
        let email = profile.email.trim();
        let phone = profile.phone.trim();
        if display_name.chars().count() > MAX_NAME_CHARS {
            return Err(Error::invalid(format!(
                "o nome passa de {MAX_NAME_CHARS} caracteres"
            )));
        }
        if !username.is_empty() && !valid_username(username) {
            return Err(Error::invalid(
                "nome de usuário: use até 32 letras, números, '_', '.' ou '-'",
            ));
        }
        if !email.is_empty() && !valid_email(email) {
            return Err(Error::invalid("e-mail em formato inválido"));
        }
        if !phone.is_empty() && !valid_phone(phone) {
            return Err(Error::invalid("telefone em formato inválido"));
        }
        self.conn.execute(
            "UPDATE profile SET display_name = ?1, username = ?2, email = ?3, phone = ?4,
                    avatar_path = ?5, updated_at = ?6 WHERE id = 1",
            params![
                display_name,
                username,
                email,
                phone,
                clean_opt(profile.avatar_path.as_deref()),
                self.now_ms()
            ],
        )?;
        self.get_profile()
    }

    // @req RF-17 RF-02
    pub fn get_settings(&self) -> Result<Settings> {
        Ok(self.conn.query_row(
            "SELECT lava_theme, volume FROM settings WHERE id = 1",
            [],
            |r| {
                Ok(Settings {
                    lava_theme: r.get(0)?,
                    volume: r.get(1)?,
                })
            },
        )?)
    }

    // @req RF-17 RF-02
    /// Aplica só os campos presentes. `volume` deve estar em 0.0..=1.0.
    pub fn update_settings(&self, patch: &SettingsPatch) -> Result<Settings> {
        if let Some(v) = patch.volume
            && !(0.0..=1.0).contains(&v)
        {
            return Err(Error::invalid("o volume deve estar entre 0.0 e 1.0"));
        }
        self.in_tx(|| {
            let cur = self.get_settings()?;
            self.conn.execute(
                "UPDATE settings SET lava_theme = ?1, volume = ?2, updated_at = ?3 WHERE id = 1",
                params![
                    patch.lava_theme.unwrap_or(cur.lava_theme),
                    patch.volume.unwrap_or(cur.volume),
                    self.now_ms()
                ],
            )?;
            self.get_settings()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LavaTheme;
    use crate::testutil::*;

    fn perfil(email: &str, phone: &str) -> Profile {
        Profile {
            display_name: "  Carlos Antunes ".into(),
            username: "carlos.antunes".into(),
            email: email.into(),
            phone: phone.into(),
            avatar_path: None,
        }
    }

    // @test RF-16
    #[test]
    fn perfil_comeca_vazio_e_atualiza() {
        let db = db();
        let p = db.get_profile().unwrap();
        assert_eq!(
            (p.display_name.as_str(), p.email.as_str(), p.avatar_path),
            ("", "", None)
        );

        let salvo = db
            .update_profile(&perfil("carlos@exemplo.com", "+55 (11) 91234-5678"))
            .unwrap();
        assert_eq!(salvo.display_name, "Carlos Antunes", "campos são aparados");
        assert_eq!(db.get_profile().unwrap(), salvo);

        // e-mail e telefone podem ser esvaziados
        let vazio = db.update_profile(&perfil("", "")).unwrap();
        assert_eq!((vazio.email.as_str(), vazio.phone.as_str()), ("", ""));
    }

    // @test RF-16
    #[test]
    fn valida_formato_tolerante_de_email_telefone_e_usuario() {
        let db = db();
        for ok in ["a@b.co", "nome+tag@sub.dominio.com.br"] {
            assert!(db.update_profile(&perfil(ok, "")).is_ok(), "{ok}");
        }
        for ruim in [
            "semarroba",
            "a@b",
            "@b.com",
            "a@@b.com",
            "a b@c.com",
            "a@.com",
            "a@b..com",
        ] {
            let e = db.update_profile(&perfil(ruim, "")).unwrap_err();
            assert_eq!(e.code(), "invalid_input", "{ruim}");
        }
        for ok in ["11912345678", "+55 11 91234-5678", "(11) 1234.5678"] {
            assert!(db.update_profile(&perfil("", ok)).is_ok(), "{ok}");
        }
        for ruim in ["123", "abc12345678", "1234567890123456"] {
            assert_eq!(
                db.update_profile(&perfil("", ruim)).unwrap_err().code(),
                "invalid_input",
                "{ruim}"
            );
        }
        let mut p = perfil("", "");
        p.username = "tem espaço".into();
        assert_eq!(db.update_profile(&p).unwrap_err().code(), "invalid_input");
        p.username = String::new();
        assert!(db.update_profile(&p).is_ok(), "usuário vazio é permitido");
        p.display_name = "x".repeat(101);
        assert_eq!(db.update_profile(&p).unwrap_err().code(), "invalid_input");
    }

    // @test RF-17
    #[test]
    fn configuracoes_tem_padrao_laranja_e_atualizam_parcialmente() {
        let db = db();
        let s = db.get_settings().unwrap();
        assert_eq!(s.lava_theme, LavaTheme::Orange);
        assert_eq!(s.volume, 0.8);

        let s = db
            .update_settings(&SettingsPatch {
                lava_theme: Some(LavaTheme::Blue),
                volume: None,
            })
            .unwrap();
        assert_eq!((s.lava_theme, s.volume), (LavaTheme::Blue, 0.8));
        let s = db
            .update_settings(&SettingsPatch {
                lava_theme: None,
                volume: Some(0.25),
            })
            .unwrap();
        assert_eq!((s.lava_theme, s.volume), (LavaTheme::Blue, 0.25));
        assert_eq!(db.update_settings(&SettingsPatch::default()).unwrap(), s);

        for v in [-0.1, 1.1, f64::NAN] {
            let r = db.update_settings(&SettingsPatch {
                lava_theme: Some(LavaTheme::Purple),
                volume: Some(v),
            });
            assert_eq!(r.unwrap_err().code(), "invalid_input");
        }
        assert_eq!(
            db.get_settings().unwrap().lava_theme,
            LavaTheme::Blue,
            "falha não grava nada"
        );
    }
}
