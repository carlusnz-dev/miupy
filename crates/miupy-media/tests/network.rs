//! Testes que batem na rede de verdade. Ignorados por padrão:
//! `cargo test -p miupy-media --test network -- --ignored --nocapture --test-threads=1`
//! Os testes de yt-dlp exigem o binário instalado.

use miupy_media::{
    DownloadRequest, MediaResolver, MetadataConfig, MetadataProvider, OnlineMetadata, YtDlpResolver,
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
