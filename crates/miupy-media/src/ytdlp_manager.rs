//! Instala e atualiza o binário oficial do yt-dlp com verificação SHA-256.

use crate::error::{MediaError, Result};
use crate::resolver::ProgressFn;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;

const RELEASE_URL: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";

// @req RF-26
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum YtDlpState {
    Missing,
    Downloading,
    Ready,
    Error,
}

// @req RF-26
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YtDlpStatus {
    pub state: YtDlpState,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
    pub checked_at: Option<u64>,
    pub progress: f32,
    pub error: Option<String>,
}

// @req RF-26
pub type YtDlpStatusFn = Arc<dyn Fn(YtDlpStatus) + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Stamp {
    version: String,
    checked_at: u64,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Diretório de binários é fornecido pela casca, normalmente `<dados>/bin`.
// @req RF-26
pub struct YtDlpManager {
    dir: PathBuf,
    release_url: String,
    http: reqwest::Client,
    lock: tokio::sync::Mutex<()>,
}

// @req RF-26
impl YtDlpManager {
    /// Cria um gerenciador que consulta a API oficial de releases do GitHub.
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self> {
        Self::with_release_url(dir, RELEASE_URL)
    }

    /// Cria o gerenciador com endpoint de release substituto, útil para testes locais.
    pub fn with_release_url(
        dir: impl Into<PathBuf>,
        release_url: impl Into<String>,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(format!(
                "miupy/{} (yt-dlp manager)",
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| MediaError::Internal(e.to_string()))?;
        Ok(Self {
            dir: dir.into(),
            release_url: release_url.into(),
            http,
            lock: tokio::sync::Mutex::new(()),
        })
    }

    /// Caminho do executável gerenciado ou de `MIUPY_YTDLP`, quando definido.
    pub fn binary_path(&self) -> PathBuf {
        if let Some(override_path) = std::env::var_os("MIUPY_YTDLP").filter(|s| !s.is_empty()) {
            return PathBuf::from(override_path);
        }
        self.dir.join(asset_name().unwrap_or("yt-dlp"))
    }

    /// Estado persistido atual no formato de `YtDlpStatus`.
    pub async fn status(&self) -> YtDlpStatus {
        let path = self.binary_path();
        let stamp = self.read_stamp().await;
        YtDlpStatus {
            state: if path.is_file() {
                YtDlpState::Ready
            } else {
                YtDlpState::Missing
            },
            version: stamp.as_ref().map(|s| s.version.clone()),
            path: path.is_file().then_some(path),
            checked_at: stamp.map(|s| s.checked_at),
            progress: 0.0,
            error: None,
        }
    }

    /// Garante a instalação e consulta atualização no máximo uma vez a cada 24 horas.
    pub async fn ensure_current(&self, progress: Option<ProgressFn>) -> Result<YtDlpStatus> {
        self.update(false, progress).await
    }

    /// Emite estados `downloading`, `ready` ou `error` junto com o progresso e a versão conhecida.
    pub async fn ensure_current_with_status(
        &self,
        callback: Option<YtDlpStatusFn>,
    ) -> Result<YtDlpStatus> {
        self.update_with_status(false, callback).await
    }

    /// Consulta a última release; `force` ignora o intervalo diário.
    pub async fn update(&self, force: bool, progress: Option<ProgressFn>) -> Result<YtDlpStatus> {
        let _guard = self.lock.lock().await;
        let path = self.binary_path();
        if std::env::var_os("MIUPY_YTDLP").is_some_and(|s| !s.is_empty()) {
            return if path.is_file() {
                Ok(self.status().await)
            } else {
                Err(MediaError::Unavailable(format!(
                    "MIUPY_YTDLP não existe: {}",
                    path.display()
                )))
            };
        }
        if !force
            && path.is_file()
            && self
                .read_stamp()
                .await
                .is_some_and(|s| now_ms().saturating_sub(s.checked_at) < 86_400_000)
        {
            return Ok(self.status().await);
        }
        let name = asset_name().ok_or_else(|| {
            MediaError::Unavailable("plataforma sem binário oficial do yt-dlp".into())
        })?;
        let release: Release = self
            .fetch(&self.release_url)
            .await?
            .json()
            .await
            .map_err(|e| MediaError::Network(e.to_string()))?;
        let asset = release
            .assets
            .iter()
            .find(|a| a.name == name)
            .ok_or_else(|| MediaError::Unavailable(format!("release sem {name}")))?;
        let checksums = release
            .assets
            .iter()
            .find(|a| a.name == "SHA2-256SUMS")
            .ok_or_else(|| MediaError::Unavailable("release sem SHA2-256SUMS".into()))?;
        let sums = self
            .fetch(&checksums.browser_download_url)
            .await?
            .text()
            .await
            .map_err(|e| MediaError::Network(e.to_string()))?;
        let expected = sums
            .lines()
            .find_map(|line| {
                let (hash, file) = line.split_once(char::is_whitespace)?;
                (file.trim().trim_start_matches('*') == name && hash.len() == 64)
                    .then_some(hash.to_ascii_lowercase())
            })
            .ok_or_else(|| MediaError::Internal(format!("checksum ausente para {name}")))?;
        if path.is_file()
            && self
                .read_stamp()
                .await
                .is_some_and(|s| s.version == release.tag_name)
        {
            self.write_stamp(&release.tag_name).await?;
            return Ok(self.status().await);
        }
        tokio::fs::create_dir_all(&self.dir).await?;
        let temp = self.dir.join(format!("{name}.part"));
        let result = self
            .download_verified(&asset.browser_download_url, &temp, &expected, progress)
            .await;
        if let Err(e) = result {
            let _ = tokio::fs::remove_file(&temp).await;
            return Err(e);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o755)).await?;
        }
        tokio::fs::rename(&temp, &path).await?;
        self.write_stamp(&release.tag_name).await?;
        Ok(self.status().await)
    }

    /// Variante que publica estados completos no formato de `YtDlpStatus`.
    pub async fn update_with_status(
        &self,
        force: bool,
        callback: Option<YtDlpStatusFn>,
    ) -> Result<YtDlpStatus> {
        let previous = self.status().await;
        let progress = callback.as_ref().map(|callback| {
            let callback = callback.clone();
            let previous = previous.clone();
            let path = self.binary_path();
            Arc::new(move |progress| {
                callback(YtDlpStatus {
                    state: YtDlpState::Downloading,
                    version: previous.version.clone(),
                    path: Some(path.clone()),
                    checked_at: previous.checked_at,
                    progress,
                    error: None,
                });
            }) as ProgressFn
        });
        match self.update(force, progress).await {
            Ok(status) => {
                if let Some(callback) = callback {
                    callback(status.clone());
                }
                Ok(status)
            }
            Err(error) => {
                if let Some(callback) = callback {
                    let mut status = previous;
                    status.state = YtDlpState::Error;
                    status.progress = 0.0;
                    status.error = Some(error.to_string());
                    callback(status);
                }
                Err(error)
            }
        }
    }

    async fn download_verified(
        &self,
        url: &str,
        temp: &Path,
        expected: &str,
        progress: Option<ProgressFn>,
    ) -> Result<()> {
        let mut response = self.fetch(url).await?;
        let total = response.content_length();
        let mut output = tokio::fs::File::create(temp).await?;
        let mut digest = Sha256::new();
        let mut received = 0u64;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| MediaError::Network(e.to_string()))?
        {
            digest.update(&chunk);
            output.write_all(&chunk).await?;
            received += chunk.len() as u64;
            if let (Some(total), Some(callback)) = (total, &progress)
                && total > 0
            {
                callback((received as f32 / total as f32).min(1.0));
            }
        }
        output.sync_all().await?;
        drop(output);
        let actual = digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if actual != expected {
            return Err(MediaError::Internal("SHA-256 do yt-dlp não confere".into()));
        }
        if let Some(callback) = progress {
            callback(1.0);
        }
        Ok(())
    }

    async fn fetch(&self, url: &str) -> Result<reqwest::Response> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| MediaError::Network(e.without_url().to_string()))?;
        if !response.status().is_success() {
            return Err(MediaError::Network(format!(
                "yt-dlp: HTTP {}",
                response.status()
            )));
        }
        Ok(response)
    }
    async fn read_stamp(&self) -> Option<Stamp> {
        let bytes = tokio::fs::read(self.dir.join("yt-dlp-status.json"))
            .await
            .ok()?;
        serde_json::from_slice(&bytes).ok()
    }
    async fn write_stamp(&self, version: &str) -> Result<()> {
        let bytes = serde_json::to_vec(&Stamp {
            version: version.into(),
            checked_at: now_ms(),
        })
        .map_err(|e| MediaError::Internal(e.to_string()))?;
        let tmp = self.dir.join("yt-dlp-status.json.part");
        tokio::fs::write(&tmp, bytes).await?;
        tokio::fs::rename(tmp, self.dir.join("yt-dlp-status.json")).await?;
        Ok(())
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn asset_name() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("yt-dlp_linux"),
        ("linux", "aarch64") => Some("yt-dlp_linux_aarch64"),
        ("macos", "x86_64" | "aarch64") => Some("yt-dlp_macos"),
        ("windows", "x86_64" | "aarch64") => Some("yt-dlp.exe"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    type Reply = (u16, Vec<u8>, Option<usize>);
    async fn server(replies: Vec<Reply>) -> (String, tokio::task::JoinHandle<usize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let mut count = 0;
            for (status, mut body, declared) in replies {
                let Ok((mut stream, _)) =
                    tokio::time::timeout(Duration::from_secs(3), listener.accept())
                        .await
                        .unwrap()
                else {
                    break;
                };
                let mut request = [0; 2048];
                let _ = stream.read(&mut request).await;
                body = String::from_utf8_lossy(&body)
                    .replace("BASE", &format!("http://{addr}"))
                    .into_bytes();
                let len = declared.unwrap_or(body.len());
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(head.as_bytes()).await.unwrap();
                stream.write_all(&body).await.unwrap();
                let _ = stream.shutdown().await;
                count += 1;
            }
            count
        });
        (format!("http://{addr}"), task)
    }

    fn release(_base: &str, name: &str) -> Vec<u8> {
        serde_json::json!({"tag_name":"2026.10.01","assets":[
            {"name":name,"browser_download_url":"BASE/binary"},
            {"name":"SHA2-256SUMS","browser_download_url":"BASE/sums"}
        ]})
        .to_string()
        .into_bytes()
    }

    // @test RF-26
    #[tokio::test]
    async fn instala_binario_com_checksum_e_preserva_anterior_em_checksum_errado() {
        let name = asset_name().unwrap();
        let bin = b"#!fake yt-dlp executable";
        let hash = Sha256::digest(bin)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let (base, task) = server(vec![
            (200, release("", name), None),
            (200, format!("{hash}  {name}\n").into_bytes(), None),
            (200, bin.to_vec(), None),
        ])
        .await;
        let dir = tempfile::tempdir().unwrap();
        let manager =
            YtDlpManager::with_release_url(dir.path(), format!("{base}/release")).unwrap();
        let states = Arc::new(Mutex::new(Vec::new()));
        let observed = states.clone();
        let status = manager
            .update_with_status(
                true,
                Some(Arc::new(move |status| {
                    observed.lock().unwrap().push(status.state)
                })),
            )
            .await
            .unwrap();
        assert_eq!(status.state, YtDlpState::Ready);
        {
            let states = states.lock().unwrap();
            assert_eq!(states.first(), Some(&YtDlpState::Downloading));
            assert_eq!(states.last(), Some(&YtDlpState::Ready));
            assert!(states.len() >= 2);
        }
        assert_eq!(std::fs::read(status.path.unwrap()).unwrap(), bin);
        assert_eq!(task.await.unwrap(), 3);

        let (base, task) = server(vec![
            (200, release("", name), None),
            (
                200,
                format!("{}  {name}\n", "0".repeat(64)).into_bytes(),
                None,
            ),
            (200, b"bad replacement".to_vec(), None),
        ])
        .await;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join(name);
        tokio::fs::write(&target, b"old valid binary")
            .await
            .unwrap();
        let manager =
            YtDlpManager::with_release_url(dir.path(), format!("{base}/release")).unwrap();
        let states = Arc::new(Mutex::new(Vec::new()));
        let observed = states.clone();
        assert!(
            manager
                .update_with_status(
                    true,
                    Some(Arc::new(move |status| observed
                        .lock()
                        .unwrap()
                        .push(status.state))),
                )
                .await
                .is_err()
        );
        assert_eq!(states.lock().unwrap().last(), Some(&YtDlpState::Error));
        assert_eq!(std::fs::read(target).unwrap(), b"old valid binary");
        assert_eq!(task.await.unwrap(), 3);
    }

    // @test RF-26
    #[tokio::test]
    async fn download_interrompido_nao_instala_e_versao_igual_nao_baixa() {
        let name = asset_name().unwrap();
        let bin = b"#!fake yt-dlp executable";
        let hash = Sha256::digest(bin)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let (base, task) = server(vec![
            (200, release("", name), None),
            (200, format!("{hash}  {name}\n").into_bytes(), None),
            (200, b"short".to_vec(), Some(50)),
        ])
        .await;
        let dir = tempfile::tempdir().unwrap();
        let manager =
            YtDlpManager::with_release_url(dir.path(), format!("{base}/release")).unwrap();
        assert!(manager.update(true, None).await.is_err());
        assert!(!dir.path().join(name).exists());
        assert_eq!(task.await.unwrap(), 3);

        // A release with the same version needs metadata requests, but never the binary request.
        let (base, task) = server(vec![
            (200, release("http://unused", name), None),
            (200, format!("{hash}  {name}\n").into_bytes(), None),
        ])
        .await;
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join(name), bin).await.unwrap();
        tokio::fs::write(
            dir.path().join("yt-dlp-status.json"),
            serde_json::json!({"version":"2026.10.01","checked_at":0}).to_string(),
        )
        .await
        .unwrap();
        let manager =
            YtDlpManager::with_release_url(dir.path(), format!("{base}/release")).unwrap();
        let status = manager.update(true, None).await.unwrap();
        assert_eq!(status.state, YtDlpState::Ready);
        assert_eq!(task.await.unwrap(), 2);
    }
}
