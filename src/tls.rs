use crate::kube::Kube;
use std::sync::Arc;
use tokio_rustls::rustls::{NoClientAuth, ResolvesServerCert, ServerConfig};
use tokio_rustls::TlsAcceptor;

pub fn create_acceptor(kube: Arc<Kube>) -> TlsAcceptor {
    let mut config = ServerConfig::new(NoClientAuth::new());
    config.set_protocols(&["h2".to_string().into(), "http/1.1".to_string().into()]);
    config.cert_resolver = Arc::new(CertificateResolver::new(kube));

    TlsAcceptor::from(Arc::new(config))
}

struct CertificateResolver {
    kube: Arc<Kube>,
}

impl CertificateResolver {
    fn new(kube: Arc<Kube>) -> Self {
        CertificateResolver { kube }
    }
}

impl ResolvesServerCert for CertificateResolver {
    fn resolve(
        &self,
        hello: tokio_rustls::rustls::ClientHello,
    ) -> Option<tokio_rustls::rustls::sign::CertifiedKey> {
        let name = match hello.server_name() {
            Some(name) => name,
            None => {
                warn!("Client does not provide a servername.");
                return None;
            }
        };

        let kube = self.kube.clone();

        futures::executor::block_on(kube.get_cert(name.into()))
    }
}
