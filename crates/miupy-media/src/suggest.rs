//! Sugestões rápidas do YouTube para a busca enquanto o usuário digita.

use crate::error::{MediaError, Result};
use std::time::Duration;

const ENDPOINT: &str = "https://suggestqueries.google.com/complete/search";

// @req RF-23
/// Devolve até dez sugestões; consulta em branco não acessa a rede.
pub async fn suggest(query: &str) -> Result<Vec<String>> {
    suggest_from(ENDPOINT, query).await
}

// @req RF-23
/// Variante com endpoint injetável para teste local.
pub async fn suggest_from(endpoint: &str, query: &str) -> Result<Vec<String>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .connect_timeout(Duration::from_secs(2))
        .user_agent(format!("miupy/{} (busca)", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| MediaError::Network(e.to_string()))?;
    let mut url = reqwest::Url::parse(endpoint)
        .map_err(|e| MediaError::Network(format!("URL de sugestões inválida: {e}")))?;
    url.query_pairs_mut()
        .append_pair("client", "firefox")
        .append_pair("ds", "yt")
        .append_pair("q", query);
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| MediaError::Network(e.without_url().to_string()))?;
    if !response.status().is_success() {
        return Err(MediaError::Network(format!(
            "sugestões: HTTP {}",
            response.status()
        )));
    }
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| MediaError::Network(format!("resposta de sugestões: {e}")))?;
    Ok(value
        .get(1)
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .take(10)
        .map(str::to_string)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    // @test RF-23
    #[tokio::test]
    async fn vazia_nao_consulta_rede_e_erro_e_network() {
        assert!(
            suggest_from("http://127.0.0.1:1", " ")
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            suggest_from("http://127.0.0.1:1", "á")
                .await
                .unwrap_err()
                .code(),
            "network"
        );
    }
    // @test RF-23
    #[test]
    fn resposta_real_com_acentos() {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/suggest_racionais.json")).unwrap();
        assert_eq!(v[1][0], "racionais música");
        assert!(v[1].as_array().unwrap().len() >= 5);
    }
}
