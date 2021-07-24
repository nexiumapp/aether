use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::rustls::internal::pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{NoClientAuth, ResolvesServerCert, ServerConfig};
use tokio_rustls::TlsAcceptor;

pub fn create_acceptor() -> TlsAcceptor {
    let certs = certs(&mut BufReader::new(
        File::open(Path::new("./certs/localhost.crt")).unwrap(),
    ))
    .unwrap();
    let mut keys = pkcs8_private_keys(&mut BufReader::new(
        File::open(Path::new("./certs/localhost.key")).unwrap(),
    ))
    .unwrap();

    let mut config = ServerConfig::new(NoClientAuth::new());
    config.set_protocols(&["h2".to_string().into()]);
    config.set_single_cert(certs, keys.remove(0)).unwrap();
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
