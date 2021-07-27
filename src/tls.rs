use std::sync::Arc;
use tokio_rustls::rustls::{NoClientAuth, ResolvesServerCert, ServerConfig};
use tokio_rustls::TlsAcceptor;

pub fn create_acceptor() -> TlsAcceptor {
    let mut config = ServerConfig::new(NoClientAuth::new());
    config.set_protocols(&["h2".to_string().into(), "http/1.1".to_string().into()]);
    config.cert_resolver = Arc::new(CertificateResolver {});

    TlsAcceptor::from(Arc::new(config))
}

struct CertificateResolver {}

impl ResolvesServerCert for CertificateResolver {
    fn resolve(
        &self,
        _hello: tokio_rustls::rustls::ClientHello,
    ) -> Option<tokio_rustls::rustls::sign::CertifiedKey> {
        todo!();
    }
}
