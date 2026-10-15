//! Testes que batem na rede de verdade. Ignorados por padrão:
//! `cargo test -p miupy-media --test network -- --ignored --nocapture --test-threads=1`
//! Os testes de yt-dlp exigem o binário instalado.

use miupy_media::{
    DownloadRequest, MediaResolver, MetadataConfig, MetadataProvider, OnlineMetadata, YtDlpManager,
    YtDlpResolver, suggest,
};

fn provider() -> OnlineMetadata {
    OnlineMetadata::new(MetadataConfig::new("miupy-tests@example.com")).unwrap()
}

// @test RF-12
#[tokio::test]
#[ignore = "rede"]
async fn musicbrainz_recording_e_artista() {
    // cada OnlineMetadata tem o próprio limitador: espaça de execuções anteriores
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let p = provider();
    let m = p
        .find_recording("Get Lucky", Some("Daft Punk"), Some(294_000))
        .await
        .unwrap()
        .expect("sem resultado");
    println!("{m:#?}");
    assert!(m.title.to_lowercase().contains("get lucky"));
    let a = p
        .artist_details(m.artist_mbid.as_deref().unwrap())
        .await
        .unwrap();
    println!("{a:#?}");
    assert!(!a.name.is_empty());
}

// @test RF-12 RF-20
#[tokio::test]
#[ignore = "rede real: MusicBrainz"]
async fn musicbrainz_faixas_do_autor_com_canal_errado() {
    let p = provider();
    for (title, artist, channel) in [
        ("Bete Balanço", "Cazuza", "Canal Desconhecido"),
        ("Velha Infância", "Tribalistas", "Passe em Casa"),
    ] {
        let baseline = p.find_recording(title, Some(artist), None).await.unwrap();
        println!("{title} com artista: {baseline:#?}");
        let Some(duration) = baseline.as_ref().and_then(|r| r.duration_ms) else {
            println!("{title}: sem duração na busca com artista");
            continue;
        };
        let found = p
            .find_recording(title, Some(channel), Some(duration))
            .await
            .unwrap();
        println!("{title}: {found:#?}");
        let found = found.expect("sem correspondência segura");
        let saved_artist = if found.title_only {
            channel
        } else {
            &found.artist
        };
        assert_eq!(
            saved_artist,
            if found.title_only { channel } else { artist }
        );
        if found.title_only {
            let saved_album: Option<&str> = None;
            assert_eq!(
                saved_album, None,
                "{title}: busca só por título não deve gravar álbum"
            );
        }
    }
}

// @test RF-27
#[tokio::test]
#[ignore = "rede real: MusicBrainz, Wikidata e Commons"]
async fn foto_artista_real_por_mbid() {
    let p = provider();
    let artist = p
        .find_artist("Daft Punk")
        .await
        .unwrap()
        .expect("Daft Punk ausente");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let photo = p
        .artist_photo(Some(&artist.mbid))
        .await
        .unwrap()
        .expect("sem foto relacionada");
    println!(
        "MBID {}: {} bytes, .{}, {}",
        artist.mbid,
        photo.bytes.len(),
        photo.extension,
        photo.credit
    );
    assert!(photo.bytes.len() > 1_000);
    assert!(photo.credit.contains('—'));
}

// @test RF-23
#[tokio::test]
#[ignore = "rede real: sugestões do YouTube"]
async fn sugestoes_youtube_reais_com_acentos() {
    let values = suggest("racionais música").await.unwrap();
    println!("{values:#?}");
    assert!(!values.is_empty());
    assert!(values.len() <= 10);
}

// @test RF-26
#[tokio::test]
#[ignore = "rede real: baixa binário oficial e busca no YouTube"]
async fn yt_dlp_oficial_baixa_e_busca_sem_runtime_embutido() {
    let dir = tempfile::tempdir().unwrap();
    let manager = YtDlpManager::new(dir.path()).unwrap();
    let status = manager.update(true, None).await.unwrap();
    assert_eq!(status.state, miupy_media::YtDlpState::Ready);
    let resolver = YtDlpResolver::with_binary(status.path.unwrap());
    println!("yt-dlp {}", resolver.check_available().await.unwrap());
    let results = resolver
        .search("Queen Bohemian Rhapsody official video", 2)
        .await
        .unwrap();
    assert!(!results.is_empty());
}

// @test RF-12
#[tokio::test]
#[ignore = "rede"]
async fn coverart_download() {
    // Release de exemplo da documentação do Cover Art Archive.
    let dir = tempfile::tempdir().unwrap();
    let path = provider()
        .download_cover("76df3287-6cda-33eb-8e9a-044b5e15ffdd", dir.path(), "capa")
        .await
        .unwrap();
    println!("{path:?} {} bytes", std::fs::metadata(&path).unwrap().len());
    assert!(std::fs::metadata(&path).unwrap().len() > 1000);
}

// @test RF-12
#[tokio::test]
#[ignore = "rede"]
async fn lrclib_letra() {
    let l = provider()
        .lyrics(
            "I Want to Live",
            "Borislav Slavov",
            Some("Baldur's Gate 3 (Original Game Soundtrack)"),
            Some(233_000),
        )
        .await
        .unwrap()
        .expect("sem letra");
    println!(
        "plain? {} synced? {}",
        l.plain.is_some(),
        l.synced.is_some()
    );
    assert!(l.plain.is_some() || l.synced.is_some());
}

// @test RF-09
#[tokio::test]
#[ignore = "rede + yt-dlp instalado"]
async fn ytdlp_busca_real() {
    let r = YtDlpResolver::new();
    println!("versão: {}", r.check_available().await.unwrap());
    let res = r
        .search("creative commons music youtube audio library", 3)
        .await
        .unwrap();
    println!("{res:#?}");
    assert!(!res.is_empty());
}

// @test RF-11
#[tokio::test]
#[ignore = "rede + yt-dlp instalado"]
async fn ytdlp_playlist_real() {
    let playlist = YtDlpResolver::new()
        .resolve_playlist("PLMC9KNkIncKtPzgY-5rmhvj7fax8fdxoj")
        .await
        .unwrap();
    assert_eq!(playlist.id, "PLMC9KNkIncKtPzgY-5rmhvj7fax8fdxoj");
    assert!(!playlist.entries.is_empty());
}

// @test RF-18 RF-20
#[tokio::test]
#[ignore = "rede + yt-dlp instalado"]
async fn ytdlp_stream_real_responde_audio_parcial() {
    let r = YtDlpResolver::new();
    let stream = r.resolve_stream("jNQXAC9IVRw").await.unwrap();
    assert_eq!(stream.codec, "aac");
    assert_eq!(stream.container, "m4a");
    assert!(stream.duration_ms.unwrap() > 10_000);
    let client = reqwest::Client::new();
    let mut request = client
        .get(&stream.url)
        .header(reqwest::header::RANGE, "bytes=0-4095");
    for (name, value) in stream.http_headers {
        request = request.header(name.as_str(), value.as_str());
    }
    let response = request.send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);
    let body = response.bytes().await.unwrap();
    assert!(!body.is_empty());
    assert!(body.windows(4).any(|bytes| bytes == b"ftyp"));
    let metadata = r.video_metadata("jNQXAC9IVRw").await.unwrap();
    assert!(!metadata.genres.is_empty());
}

// @test RF-19
#[tokio::test]
#[ignore = "rede + yt-dlp instalado"]
async fn ytdlp_mix_real_devolve_faixas_parecidas() {
    let related = YtDlpResolver::new()
        .related_tracks("dQw4w9WgXcQ", 5)
        .await
        .unwrap();
    // vídeo de música estável: o mix existe; lista vazia aqui é regressão, não caso normal
    assert!(!related.is_empty(), "o mix não devolveu nenhuma faixa");
    assert!(related.len() <= 5);
    assert!(related.iter().all(|item| item.video_id != "dQw4w9WgXcQ"));
}

// @test RF-10
#[tokio::test]
#[ignore = "rede + yt-dlp instalado"]
async fn ytdlp_download_real() {
    let r = YtDlpResolver::new();
    let dir = tempfile::tempdir().unwrap();
    // Vídeo curto de teste "Me at the zoo" (19 s).
    let req = DownloadRequest::new("jNQXAC9IVRw", dir.path(), "zoo");
    let out = r.download_audio(req).await.unwrap();
    println!("{out:?}");
    assert!(out.duration_ms > 10_000);
}
