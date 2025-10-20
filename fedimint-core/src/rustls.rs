use std::sync::OnceLock;

static INSTALL_CRYPTO: OnceLock<()> = OnceLock::new();

#[cfg(not(target_family = "wasm"))]
pub fn install_crypto_provider() {
    use fedimint_logging::LOG_CORE;
    use tracing::warn;

    INSTALL_CRYPTO.get_or_init(|| {
        if tokio_rustls::rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .is_err()
        {
            warn!(
                target: LOG_CORE,
                "Failed to install rustls crypto provider. Hopefully harmless."
            );
        }
    });
}
