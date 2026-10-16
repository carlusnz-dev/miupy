//! Limpeza de títulos de vídeo do YouTube para melhorar a consulta ao MusicBrainz.
//!
//! Função pura, sem E/S. Heurística: não existe regra perfeita, então o resultado é um palpite
//! que a camada acima pode descartar se o MusicBrainz não confirmar.

/// Resultado de [`clean_title`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedTitle {
    /// Artista principal, se foi possível inferir (do título ou do canal).
    pub artist: Option<String>,
    /// Título da música, sem ruído de vídeo e sem "feat.".
    pub title: String,
    /// Participações ("feat. X", "ft. X", "with X" entre parênteses).
    pub featured: Vec<String>,
}

impl ParsedTitle {
    /// Crédito exibido: participações ficam sempre junto do artista.
    // @req RF-09 RF-12
    pub fn artist_credit(&self) -> Option<String> {
        self.artist.as_ref().map(|artist| {
            if self.featured.is_empty() {
                artist.clone()
            } else {
                format!("{artist} feat. {}", self.featured.join(", "))
            }
        })
    }
}

/// Palavras que marcam um trecho entre parênteses/colchetes como ruído de vídeo.
/// Risco conhecido: "(Video Game Remix)" também seria removido.
const NOISE_WORDS: &[&str] = &[
    "official",
    "oficial",
    "lyrics",
    "lyric",
    "letra",
    "letras",
    "audio",
    "áudio",
    "video",
    "vídeo",
    "videoclip",
    "videoclipe",
    "clipe",
    "visualizer",
    "visualiser",
    "4k",
    "8k",
    "hd",
    "hq",
    "uhd",
    "1080p",
    "720p",
    "mv",
    "remastered",
    "remaster",
    "legendado",
    "tradução",
    "traducao",
    "explicit",
];

/// Ruído que aparece solto no fim do título ("Song Official Video").
const TRAILING_NOISE: &[&str] = &[
    "official",
    "video",
    "lyrics",
    "lyric",
    "audio",
    "hd",
    "hq",
    "4k",
    "visualizer",
    "mv",
];

const FEAT_PREFIXES: &[&str] = &["feat.", "feat", "ft.", "ft", "featuring", "with", "com"];

// @req RF-12
/// Extrai artista e título de um título de vídeo e do nome do canal.
///
/// ```
/// use miupy_media::title::clean_title;
/// let p = clean_title("Daft Punk - Get Lucky (Official Video) [4K]", "Daft Punk");
/// assert_eq!(p.artist.as_deref(), Some("Daft Punk"));
/// assert_eq!(p.title, "Get Lucky");
/// ```
pub fn clean_title(raw_title: &str, channel: &str) -> ParsedTitle {
    clean_title_with_artists(raw_title, channel, &[])
}

// @req RF-09 RF-12
/// Usa artistas da discoteca para desempatar a ordem quando o canal não identifica nenhum lado.
pub fn clean_title_with_artists(
    raw_title: &str,
    channel: &str,
    known_artists: &[String],
) -> ParsedTitle {
    let (channel_artist, is_topic) = clean_channel(channel);
    let mut featured = Vec::new();

    let collapsed = collapse_ws(raw_title);
    // Segmentos separados por " | ": descarta os que são só ruído e fica com o primeiro.
    let main = collapsed
        .split(" | ")
        .map(|seg| strip_trailing_noise(&strip_groups(seg, &mut Vec::new())))
        .zip(collapsed.split(" | "))
        .find(|(cleaned, _)| !cleaned.is_empty())
        .map(|(_, raw)| raw.to_string())
        .unwrap_or_else(|| collapsed.clone());

    let no_groups = strip_groups(&main, &mut featured);
    let no_noise = strip_trailing_noise(&no_groups);

    let (mut left, mut right) = match split_artist_title(&no_noise) {
        Some((a, t)) => (Some(a), t),
        None => (None, no_noise.clone()),
    };

    // @req RF-09 RF-20
    // Três partes costumam ser "artista - álbum - faixa". O canal confirma o primeiro
    // lado; mesmo sem ele, o último lado é a faixa, nunca o álbum intermediário.
    let parts: Vec<&str> = no_noise.split(" - ").map(str::trim).collect();
    if parts.len() >= 3
        && parts.iter().all(|p| !p.is_empty())
        && !matches!(
            parts[parts.len() - 1].to_lowercase().as_str(),
            "ao vivo" | "live"
        )
        && channel_artist
            .as_deref()
            .is_some_and(|ch| channel_matches(parts[0], ch, channel))
    {
        left = channel_artist.clone();
        right = parts[parts.len() - 1].to_string();
    }

    // "Música - Artista" quando o canal é o lado direito.
    if let (Some(a), Some(ch)) = (&left, &channel_artist)
        && channel_matches(&right, ch, channel)
        && !channel_matches(a, ch, channel)
    {
        let (a, t) = (a.clone(), right.clone());
        left = Some(t);
        right = a;
    }

    if let Some(a) = &left {
        let left_key = artist_key(a);
        let left_known = known_artists.iter().any(|known| {
            artist_matches(a, known) || left_key.starts_with(&format!("{} ", artist_key(known)))
        });
        let right_known = known_artists
            .iter()
            .any(|known| artist_key(&right) == artist_key(known));
        let channel_decides = channel_artist.as_deref().is_some_and(|ch| {
            channel_matches(a, ch, channel) || channel_matches(&right, ch, channel)
        });
        let name_like = a.split_whitespace().count() >= 2
            && a.split_whitespace().all(|word| {
                word.chars().count() > 1 && word.chars().next().is_some_and(char::is_uppercase)
            });
        if !channel_decides && right_known && !left_known && !name_like {
            let old_left = a.clone();
            left = Some(right.clone());
            right = old_left;
        }
    }

    // Canal do artista conhecido: se nenhum lado é o canal e o segundo lado tem
    // aparência de nome de álbum, o primeiro é a faixa.
    if parts.len() == 2
        && let (Some(ch), Some(a)) = (&channel_artist, &left)
        && !eq_ci(a, ch)
        && !eq_ci(&right, ch)
        && right.split_whitespace().count() >= 6
        && a.split_whitespace().count() <= 5
    {
        let track = a.clone();
        left = Some(ch.clone());
        right = track;
    }

    // "Artista feat. X" / "Música ft. X" sem parênteses.
    let mut artist = left.map(|a| extract_inline_feat(&a, &mut featured));
    let mut title = extract_inline_feat(&right, &mut featured);

    if artist.is_none() {
        artist = channel_artist.clone();
    }
    // Canal "Artista - Topic" que repete o artista no início do título.
    if is_topic
        && let Some(a) = &artist
        && let Some(rest) = strip_prefix_ci(&title, a)
        && let Some(rest) = rest
            .strip_prefix(" - ")
            .or_else(|| rest.strip_prefix(" – "))
    {
        title = rest.trim().to_string();
    }

    let title = unquote(&title);
    let title = if title.is_empty() {
        collapsed.clone()
    } else {
        title
    };
    let artist = artist.map(|a| unquote(&a)).filter(|a| !a.is_empty());

    featured.retain(|f| !f.is_empty());
    featured.dedup();
    ParsedTitle {
        artist,
        title,
        featured,
    }
}

/// Retorna `(artista, é_topic)`. "Artista - Topic" e "ArtistaVEVO" viram "Artista".
fn clean_channel(channel: &str) -> (Option<String>, bool) {
    let c = collapse_ws(channel);
    if c.is_empty() {
        return (None, false);
    }
    if let Some(base) = strip_suffix_ci(&c, " - topic") {
        return (non_empty(base.trim()), true);
    }
    let mut name = c.as_str();
    for suffix in [" official", " oficial", "vevo", " vevo"] {
        if let Some(base) = strip_suffix_ci(name, suffix)
            && !base.trim().is_empty()
        {
            name = base.trim();
            break;
        }
    }
    if let Some(base) = strip_suffix_ci(name, " tv")
        && !base.trim().is_empty()
    {
        name = base.trim();
    }
    (non_empty(name), false)
}

fn non_empty(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn eq_ci(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

fn artist_key(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => Some('a'),
            'é' | 'è' | 'ê' | 'ë' => Some('e'),
            'í' | 'ì' | 'î' | 'ï' => Some('i'),
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => Some('o'),
            'ú' | 'ù' | 'û' | 'ü' => Some('u'),
            'ç' => Some('c'),
            c if c.is_alphanumeric() => Some(c),
            c if c.is_whitespace() => Some(' '),
            _ => None,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn artist_matches(a: &str, b: &str) -> bool {
    let a = artist_key(a);
    let b = artist_key(b);
    !a.is_empty() && !b.is_empty() && (a == b || prefix_matches(&a, &b) || prefix_matches(&b, &a))
}

fn prefix_matches(short: &str, long: &str) -> bool {
    short.split_whitespace().count() >= 2 && long.starts_with(&format!("{short} "))
}

fn channel_matches(artist: &str, cleaned: &str, raw: &str) -> bool {
    if artist_matches(artist, cleaned) {
        return true;
    }
    // @req RF-09 RF-12: sufixos de canal não fazem parte do nome do artista.
    let raw_key = artist_key(raw);
    let cleaned_key = artist_key(cleaned);
    let generic_suffix = [
        "tv", "music", "official", "records", "channel", "hits", "vevo",
    ]
    .iter()
    .any(|suffix| raw_key == format!("{cleaned_key} {suffix}"));
    generic_suffix && artist_key(artist).starts_with(&format!("{cleaned_key} "))
}

fn strip_suffix_ci<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
    let (ls, lf) = (s.to_lowercase(), suffix.to_lowercase());
    if ls.ends_with(&lf) && s.is_char_boundary(s.len() - lf.len()) {
        Some(&s[..s.len() - lf.len()])
    } else {
        None
    }
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let n = prefix.len();
    (s.len() >= n && s.is_char_boundary(n) && s[..n].to_lowercase() == prefix.to_lowercase())
        .then(|| &s[n..])
}

fn closer(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '【' => Some('】'),
        '「' => Some('」'),
        '『' => Some('』'),
        _ => None,
    }
}

fn words(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
}

fn is_noise_group(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("melhores momentos")
        || lower.ends_with(" ao vivo")
        || words(content).any(|w| NOISE_WORDS.contains(&w.as_str()))
}

/// Se o conteúdo começa com "feat."/"ft."/"with", devolve os convidados.
fn parse_feat(content: &str) -> Option<Vec<String>> {
    let trimmed = content.trim();
    let lower = trimmed.to_lowercase();
    for p in FEAT_PREFIXES {
        if let Some(rest) = lower.strip_prefix(p)
            && rest.starts_with([' ', '.'])
        {
            let rest = trimmed[p.len()..].trim_start_matches(['.', ' ']);
            return Some(split_guests(rest));
        }
    }
    None
}

fn split_guests(s: &str) -> Vec<String> {
    s.replace(" & ", ",")
        .replace(" and ", ",")
        .replace(" e ", ",")
        .replace(" x ", ",")
        .split(',')
        .map(|g| g.trim().to_string())
        .filter(|g| !g.is_empty())
        .collect()
}

/// Remove grupos `(...)`/`[...]` que são ruído ou "feat.", preservando os demais
/// ("(Acoustic)", "(Remix)"). Guests encontrados vão para `featured`.
fn strip_groups(s: &str, featured: &mut Vec<String>) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if let Some(close) = closer(c)
            && let Some(rel) = chars[i + 1..].iter().position(|&x| x == close)
        {
            let end = i + 1 + rel;
            let content: String = chars[i + 1..end].iter().collect();
            if let Some(guests) = parse_feat(&content) {
                featured.extend(guests);
                i = end + 1;
                continue;
            }
            if is_noise_group(&content) {
                i = end + 1;
                continue;
            }
            out.extend(&chars[i..=end]);
            i = end + 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    collapse_ws(&out)
}

/// Remove ruído solto no fim ("... Official Video", "... HD") e separadores pendurados.
fn strip_trailing_noise(s: &str) -> String {
    let mut tokens: Vec<&str> = s.split_whitespace().collect();
    // @req RF-12
    if tokens.len() > 2
        && !matches!(tokens[tokens.len() - 3], "-" | "–" | "—")
        && tokens[tokens.len() - 2..]
            .join(" ")
            .eq_ignore_ascii_case("Ao Vivo")
    {
        tokens.truncate(tokens.len() - 2);
    }
    let mut stripped_video = false;
    while let Some(last) = tokens.last() {
        let l = last
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if l.is_empty() {
            // separador solto ("-", "|", "–")
            tokens.pop();
        } else if TRAILING_NOISE.contains(&l.as_str()) && tokens.len() > 1 {
            stripped_video |= matches!(l.as_str(), "video" | "lyrics" | "lyric" | "audio");
            tokens.pop();
        } else if l == "music" && stripped_video && tokens.len() > 1 {
            tokens.pop();
        } else {
            break;
        }
    }
    tokens.join(" ")
}

/// Divide no primeiro separador com espaços: " - ", " – ", " — ".
fn split_artist_title(s: &str) -> Option<(String, String)> {
    let position = [" - ", " – ", " — "]
        .iter()
        .filter_map(|sep| s.find(sep).map(|pos| (pos, *sep)))
        .min_by_key(|(pos, _)| *pos)?;
    let (a, t) = s.split_at(position.0);
    let t = &t[position.1.len()..];
    let (a, t) = (a.trim(), t.trim());
    (!a.is_empty() && !t.is_empty()).then(|| (a.to_string(), t.to_string()))
}

/// "Música ft. X" no fim, sem parênteses. Devolve o texto sem o trecho.
fn extract_inline_feat(s: &str, featured: &mut Vec<String>) -> String {
    let lower = s.to_lowercase();
    for p in ["feat.", "feat ", "ft.", "ft ", "featuring "] {
        let needle = format!(" {p}");
        if let Some(pos) = lower.find(&needle)
            && s.is_char_boundary(pos)
        {
            let guests = &s[pos + needle.len()..];
            featured.extend(split_guests(guests));
            return s[..pos].trim().to_string();
        }
    }
    s.trim().to_string()
}

fn unquote(s: &str) -> String {
    s.trim()
        .trim_matches(|c| matches!(c, '"' | '\'' | '“' | '”' | '‘' | '’' | '«' | '»'))
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(title: &str, channel: &str) -> (Option<String>, String, Vec<String>) {
        let r = clean_title(title, channel);
        (r.artist, r.title, r.featured)
    }

    // @test RF-09 RF-12
    #[test]
    fn casos_reais_de_ordem_canal_ruido_e_participacao() {
        let cases = [
            (
                "Racionais MC's - Negro Drama (Racionais 3 Décadas Ao Vivo)",
                "Racionais TV",
                "Racionais MC's",
                "Negro Drama",
                &[][..],
            ),
            (
                "Racionais Mc's - Negro Drama (Letras)",
                "MiNi MaX",
                "Racionais Mc's",
                "Negro Drama",
                &[][..],
            ),
            (
                "Racionais Mc's - Negro Drama (Letra)",
                "MiNi MaX",
                "Racionais Mc's",
                "Negro Drama",
                &[][..],
            ),
            (
                "Racionais Mc's - Negro Drama (Lyrics)",
                "MiNi MaX",
                "Racionais Mc's",
                "Negro Drama",
                &[][..],
            ),
            (
                "Racionais MC's - Negro Drama",
                "Racionais",
                "Racionais MC's",
                "Negro Drama",
                &[][..],
            ),
            (
                "Racionais - Negro Drama",
                "Racionais MC's",
                "Racionais",
                "Negro Drama",
                &[][..],
            ),
            (
                "NEGRO DRAMA - RACIONAIS MC'S",
                "Racionais TV",
                "RACIONAIS MC'S",
                "NEGRO DRAMA",
                &[][..],
            ),
            (
                "Racionais MC's – Negro Drama (Clipe Oficial)",
                "Canal de Rap",
                "Racionais MC's",
                "Negro Drama",
                &[][..],
            ),
            ("Matuê - Kenny G", "Matuê", "Matuê", "Kenny G", &[][..]),
            ("Kenny G - Matuê", "MatueVEVO", "Matuê", "Kenny G", &[][..]),
            (
                "Kenny G - Matuê",
                "Matuê - Topic",
                "Matuê",
                "Kenny G",
                &[][..],
            ),
            (
                "Matuê - Kenny G (feat. X)",
                "Matuê",
                "Matuê",
                "Kenny G",
                &["X"][..],
            ),
            ("Kenny G", "Matuê - Topic", "Matuê", "Kenny G", &[][..]),
            (
                "Racionais - Negro Drama - Ao Vivo",
                "Racionais",
                "Racionais",
                "Negro Drama - Ao Vivo",
                &[][..],
            ),
            (
                "Artista — Faixa - Parte 2",
                "Canal",
                "Artista",
                "Faixa - Parte 2",
                &[][..],
            ),
        ];
        for (raw, channel, artist, title, featured) in cases {
            let got = clean_title(raw, channel);
            assert_eq!(got.artist.as_deref(), Some(artist), "{raw} / {channel}");
            assert_eq!(got.title, title, "{raw} / {channel}");
            assert_eq!(
                got.featured.iter().map(String::as_str).collect::<Vec<_>>(),
                featured,
                "{raw} / {channel}"
            );
        }
    }

    // @test RF-09
    #[test]
    fn artista_da_discoteca_desempata_canal_de_terceiro() {
        let known = vec!["Matuê".to_string()];
        let got = clean_title_with_artists("Kenny G - Matue", "Canal de Rap", &known);
        assert_eq!(got.artist.as_deref(), Some("Matue"));
        assert_eq!(got.title, "Kenny G");
    }

    // @test RF-09 RF-12
    #[test]
    fn discoteca_nao_inverte_artista_correto_nem_prefixo_de_faixa() {
        for (raw, known, title, artist) in [
            (
                "Seu Jorge - Michael Jackson",
                "Michael Jackson",
                "Michael Jackson",
                "Seu Jorge",
            ),
            (
                "Queen Of The Night - Queen",
                "Queen",
                "Queen",
                "Queen Of The Night",
            ),
        ] {
            let got = clean_title_with_artists(raw, "Canal de terceiro", &[known.into()]);
            assert_eq!(
                (got.title.as_str(), got.artist.as_deref()),
                (title, Some(artist))
            );
        }
    }

    // @test RF-09 RF-12
    #[test]
    fn resultados_reais_com_artistas_dos_outros_videos() {
        for (raw, channel, known, title, artist) in [
            (
                "Racionais MC's - Negro Drama (Racionais 3 Décadas Ao Vivo)",
                "Racionais TV",
                "Racionais Mc's",
                "Negro Drama",
                "Racionais MC's",
            ),
            (
                "Negro drama - Racionais Mcs",
                "Andre Coutinho",
                "Racionais MC's",
                "Negro drama",
                "Racionais Mcs",
            ),
            (
                "Racionais Mc's - Negro Drama (Letras)",
                "MiNi MaX",
                "Racionais MC's",
                "Negro Drama",
                "Racionais Mc's",
            ),
            (
                "Racionais Mc's - Negro Drama (Letra)",
                "MiNi MaX",
                "Racionais MC's",
                "Negro Drama",
                "Racionais Mc's",
            ),
            (
                "Racionais Mc's - Negro Drama (Lyrics)",
                "MiNi MaX",
                "Racionais MC's",
                "Negro Drama",
                "Racionais Mc's",
            ),
        ] {
            let got = clean_title_with_artists(raw, channel, &[known.to_string()]);
            assert_eq!(got.title, title, "{raw}");
            assert_eq!(got.artist.as_deref(), Some(artist), "{raw}");
        }
    }

    // @test RF-09 RF-12
    #[test]
    fn participacoes_entram_no_credito_do_artista() {
        for raw in ["Matuê feat. X - Kenny G", "Matuê - Kenny G (ft. X)"] {
            let got = clean_title(raw, "Matuê");
            assert_eq!(got.artist_credit().as_deref(), Some("Matuê feat. X"));
            assert_eq!(got.title, "Kenny G");
        }
    }

    // @test RF-12
    #[test]
    fn artista_traco_musica_com_ruido() {
        let (a, t, f) = p("Daft Punk - Get Lucky (Official Video) [4K]", "Daft Punk");
        assert_eq!(a.as_deref(), Some("Daft Punk"));
        assert_eq!(t, "Get Lucky");
        assert!(f.is_empty());
    }

    // @test RF-12
    #[test]
    fn limpa_coletanea_entre_aspas_e_ao_vivo() {
        let (a, t, _) = p(
            "Cazuza - Bete Balanço (\"Cazuza & BV - Melhores Momentos\")",
            "Canal",
        );
        assert_eq!(a.as_deref(), Some("Cazuza"));
        assert_eq!(t, "Bete Balanço");
        let (a, t, _) = p("Tribalistas - Velha Infância Ao Vivo", "Passe em Casa");
        assert_eq!(a.as_deref(), Some("Tribalistas"));
        assert_eq!(t, "Velha Infância");
        let (a, t, _) = p("Alee & Neckklace - Encontro", "Alee");
        assert_eq!(a.as_deref(), Some("Alee & Neckklace"));
        assert_eq!(t, "Encontro");
    }

    // @test RF-12
    #[test]
    fn feat_entre_parenteses_vai_para_featured() {
        let (a, t, f) = p(
            "Calvin Harris - Summer (feat. Rag'n'Bone Man)",
            "Calvin Harris",
        );
        assert_eq!(a.as_deref(), Some("Calvin Harris"));
        assert_eq!(t, "Summer");
        assert_eq!(f, vec!["Rag'n'Bone Man"]);
    }

    // @test RF-12
    #[test]
    fn sem_artista_no_titulo_usa_canal() {
        let (a, t, f) = p("Música (feat. X)", "Artista Oficial");
        assert_eq!(a.as_deref(), Some("Artista"));
        assert_eq!(t, "Música");
        assert_eq!(f, vec!["X"]);
    }

    // @test RF-12
    #[test]
    fn canal_topic() {
        let (a, t, _) = p("Bohemian Rhapsody", "Queen - Topic");
        assert_eq!(a.as_deref(), Some("Queen"));
        assert_eq!(t, "Bohemian Rhapsody");
    }

    // @test RF-12
    #[test]
    fn canal_topic_com_artista_repetido_no_titulo() {
        let (a, t, _) = p("Queen - Bohemian Rhapsody", "Queen - Topic");
        assert_eq!(a.as_deref(), Some("Queen"));
        assert_eq!(t, "Bohemian Rhapsody");
    }

    // @test RF-12
    #[test]
    fn canal_vevo() {
        let (a, t, _) = p("Shake It Off", "TaylorSwiftVEVO");
        assert_eq!(a.as_deref(), Some("TaylorSwift"));
        assert_eq!(t, "Shake It Off");
        let (a, _, _) = p("Hello", "Adele VEVO");
        assert_eq!(a.as_deref(), Some("Adele"));
    }

    // @test RF-12
    #[test]
    fn ruido_solto_no_fim() {
        let (a, t, _) = p("Coldplay - Yellow Official Video", "Coldplay");
        assert_eq!(a.as_deref(), Some("Coldplay"));
        assert_eq!(t, "Yellow");
        let (_, t, _) = p("Coldplay - Yellow Official Music Video HD", "x");
        assert_eq!(t, "Yellow");
    }

    // @test RF-12
    #[test]
    fn lyrics_audio_remastered() {
        let (_, t, _) = p("A - B (Lyrics)", "c");
        assert_eq!(t, "B");
        let (_, t, _) = p("A - B [Official Audio]", "c");
        assert_eq!(t, "B");
        let (_, t, _) = p("A - B (Remastered 2011)", "c");
        assert_eq!(t, "B");
        let (_, t, _) = p("A - B 【Official MV】", "c");
        assert_eq!(t, "B");
    }

    // @test RF-12
    #[test]
    fn grupo_que_nao_e_ruido_e_preservado() {
        let (_, t, _) = p("Nirvana - Come as You Are (Acoustic)", "Nirvana");
        assert_eq!(t, "Come as You Are (Acoustic)");
        let (_, t, _) = p("A - B (Live at Wembley)", "A");
        assert_eq!(t, "B (Live at Wembley)");
    }

    // @test RF-12
    #[test]
    fn feat_inline_sem_parenteses() {
        let (a, t, f) = p("Artista ft. Convidado - Música", "Canal");
        assert_eq!(a.as_deref(), Some("Artista"));
        assert_eq!(t, "Música");
        assert_eq!(f, vec!["Convidado"]);
        let (_, t, f) = p("Artista - Música feat. A & B", "Canal");
        assert_eq!(t, "Música");
        assert_eq!(f, vec!["A", "B"]);
    }

    // @test RF-12
    #[test]
    fn ordem_invertida_quando_canal_e_o_lado_direito() {
        let (a, t, _) = p("Música - Artista", "Artista");
        assert_eq!(a.as_deref(), Some("Artista"));
        assert_eq!(t, "Música");
    }

    // @test RF-12
    #[test]
    fn travessoes_unicode_e_aspas() {
        let (a, t, _) = p("Artista – \"Música\"", "c");
        assert_eq!(a.as_deref(), Some("Artista"));
        assert_eq!(t, "Música");
        let (a, t, _) = p("Artista — Música", "c");
        assert_eq!(a.as_deref(), Some("Artista"));
        assert_eq!(t, "Música");
    }

    // @test RF-12
    #[test]
    fn segundo_traco_fica_no_titulo() {
        let (a, t, _) = p("A - B - Ao Vivo", "c");
        assert_eq!(a.as_deref(), Some("A"));
        assert_eq!(t, "B - Ao Vivo");
    }

    // @test RF-12
    #[test]
    fn pipe_com_segmento_de_ruido() {
        let (a, t, _) = p("Artista - Música | Official Video", "c");
        assert_eq!(a.as_deref(), Some("Artista"));
        assert_eq!(t, "Música");
    }

    // @test RF-12
    #[test]
    fn hifen_sem_espacos_nao_divide() {
        let (a, t, _) = p("Jay-Z", "Canal");
        assert_eq!(a.as_deref(), Some("Canal"));
        assert_eq!(t, "Jay-Z");
    }

    // @test RF-12
    #[test]
    fn entradas_degeneradas_nao_quebram() {
        let r = clean_title("", "");
        assert_eq!(r.title, "");
        assert_eq!(r.artist, None);
        let r = clean_title("(Official Video)", "Canal");
        assert_eq!(r.title, "(Official Video)");
        let r = clean_title("   ", "  ");
        assert_eq!(r.artist, None);
        // não pode dar pânico com unicode ou parêntese sem fechar
        let _ = clean_title("日本語 - 曲 (unclosed", "チャンネル - Topic");
        let _ = clean_title("ÀÉ - ÇÃ ((", "É");
    }

    // @test RF-12
    #[test]
    fn espacos_repetidos() {
        let (a, t, _) = p("  Artista   -   Música  ", "c");
        assert_eq!(a.as_deref(), Some("Artista"));
        assert_eq!(t, "Música");
    }
}
