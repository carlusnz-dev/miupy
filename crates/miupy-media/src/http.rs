// @req RF-45
/// Evita o verificador de plataforma do reqwest, que exige inicialização JNI no Android.
pub(crate) fn client_builder() -> reqwest::ClientBuilder {
    let builder = reqwest::Client::builder();
    #[cfg(target_os = "android")]
    {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        builder.use_preconfigured_tls(config)
    }
    #[cfg(not(target_os = "android"))]
    {
        builder
    }
}
