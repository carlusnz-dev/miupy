//! Normalização de texto: busca FTS5, chave de artista e ordenação sem acento.

use rusqlite::Connection;

fn fold_char(c: char) -> char {
    match c {
        'à'..='å' | 'ā' | 'ă' | 'ą' => 'a',
        'ç' | 'ć' | 'č' => 'c',
        'ď' | 'đ' => 'd',
        'è'..='ë' | 'ē' | 'ę' | 'ě' => 'e',
        'ì'..='ï' | 'ī' | 'ı' => 'i',
        'ñ' | 'ń' | 'ň' => 'n',
        'ò'..='ö' | 'ø' | 'ō' | 'ő' => 'o',
        'ù'..='ü' | 'ū' | 'ů' | 'ű' => 'u',
        'ý' | 'ÿ' => 'y',
        'ś' | 'š' | 'ş' => 's',
        'ź' | 'ż' | 'ž' => 'z',
        'ł' => 'l',
        'ř' => 'r',
        'ť' => 't',
        other => other,
    }
}

/// Minúsculas e sem acento (letras latinas comuns). Usado na chave do artista e na
/// collation de ordenação.
pub(crate) fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(fold_char)
        .collect()
}

/// Chave de identidade de um nome (artista): aparado, espaços colapsados, sem caixa/acento.
pub(crate) fn name_key(name: &str) -> String {
    fold(&name.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// Converte o texto digitado em uma expressão FTS5 segura: cada palavra vira um prefixo
/// entre aspas e todas precisam casar (`"mus"* "ba"*`). `None` = nada pesquisável
/// (só pontuação).
pub(crate) fn fts_query(search: &str) -> Option<String> {
    let tokens: Vec<String> = search
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}

/// Registra a collation `miupy_nocase` (ordem sem caixa e sem acento).
pub(crate) fn register_collation(conn: &Connection) -> rusqlite::Result<()> {
    conn.create_collation("miupy_nocase", |a, b| fold(a).cmp(&fold(b)))
}

/// Texto opcional aparado; vazio vira `None`.
pub(crate) fn clean_opt(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    // @test RF-08
    #[test]
    fn fold_remove_acento_e_caixa() {
        assert_eq!(fold("Árvore DE Maçã"), "arvore de maca");
        assert_eq!(name_key("  Sigur   Rós "), "sigur ros");
    }

    // @test RF-08
    #[test]
    fn fts_query_sanitiza() {
        assert_eq!(
            fts_query("bohemian rhap").unwrap(),
            "\"bohemian\"* \"rhap\"*"
        );
        assert_eq!(fts_query("a\" OR b*").unwrap(), "\"a\"* \"OR\"* \"b\"*");
        assert_eq!(fts_query("  !!! "), None);
    }
}
