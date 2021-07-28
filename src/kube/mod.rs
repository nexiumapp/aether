use futures_util::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::api::networking;
use kube::api::{Api, ListParams, WatchEvent};
use kube::{Client, Error};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::BufReader;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tokio_rustls::rustls::internal::pemfile::{certs, rsa_private_keys};
use tokio_rustls::rustls::sign::{CertifiedKey, RSASigningKey};

mod state;

pub struct Kube {
    client: Client,
    ingress: Arc<RwLock<Option<state::Ingress>>>,
    certificates: Arc<RwLock<HashMap<String, CertifiedKey>>>,
}

impl Kube {
    pub async fn connect() -> Result<Self, Error> {
        let client = Client::try_default().await?;

        Ok(Kube {
            client,
            ingress: Arc::new(RwLock::new(None)),
            certificates: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn watch_ingress(&self) {
        let ingress: Api<networking::v1::Ingress> = Api::all(self.client.clone());

        let params = ListParams::default();
        let mut stream = ingress.watch(&params, "0").await.unwrap().boxed();

        while let Some(status) = stream.try_next().await.unwrap() {
            let res = match status {
                WatchEvent::Added(ingress) => self.set_ingress(ingress).await,
                WatchEvent::Modified(ingress) => self.set_ingress(ingress).await,
                WatchEvent::Deleted(ingress) => self.remove_ingress(ingress).await,
                WatchEvent::Error(e) => Err(state::IngressParseError::WatchError(e)),
                _ => Ok(()),
            };

            if let Err(e) = res {
                debug!("{}", e);
            }
        }
    }

    pub async fn watch_certificates(&self) {
        let secrets: Api<Secret> = Api::all(self.client.clone());

        let params = ListParams::default();
        let mut stream = secrets.watch(&params, "0").await.unwrap().boxed();

        while let Some(status) = stream.try_next().await.unwrap() {
            let res = match status {
                WatchEvent::Added(s) => self.set_secret(s).await,
                WatchEvent::Modified(s) => self.set_secret(s).await,
                WatchEvent::Deleted(s) => self.remove_secret(s).await,
                WatchEvent::Error(e) => Err(state::SecretParseError::WatchError(e)),
                _ => Ok(()),
            };

            if let Err(e) = res {
                debug!("{}", e);
            }
        }
    }

    pub async fn get_authority(
        &self,
        virt_host: Option<&str>,
        req_path: &str,
    ) -> Result<(String, u32), AuthorityError> {
        let guard = self.ingress.read().await;
        if guard.is_none() {
            return Err(AuthorityError::NoIngress);
        }

        let ingress = guard.as_ref().unwrap();

        let mut backend = ingress.default_route.as_ref();

        for rule in &ingress.rules {
            let host_matches = match &rule.host {
                Some(host) => match virt_host {
                    Some(virt_host) => virt_host.eq(host.as_str()),
                    None => true,
                },
                None => true,
            };

            if !host_matches {
                continue;
            }

            for path in &rule.paths {
                if req_path.starts_with(path.path.as_str()) {
                    backend = Some(&path.backend);
                }
            }
        }

        match backend {
            Some(backend) => {
                let host = format!("{}.{}", backend.name, ingress.namespace);
                Ok((host, backend.port))
            }
            None => Err(AuthorityError::NoBackend),
        }
    }

    pub async fn get_cert(&self, host: &str) -> Option<CertifiedKey> {
        let kube = self.certificates.read().await;

        kube.get(host).map(|key| key.clone())
    }

    async fn set_ingress(
        &self,
        ingress: networking::v1::Ingress,
    ) -> Result<(), state::IngressParseError> {
        let class = ingress
            .metadata
            .annotations
            .get("kubernetes.io/ingress.class");
        if class.is_none() || !class.unwrap().eq("aether") {
            debug!(
                "Received update from unrelated ingress `{:?}`, ignoring",
                class
            );
            return Ok(());
        }

        debug!("Replacing ingress config with {:?}.", class);

        let ingress: state::Ingress = TryFrom::try_from(ingress)?;
        *self.ingress.write().await = Some(ingress);

        Ok(())
    }

    async fn remove_ingress(
        &self,
        ingress: networking::v1::Ingress,
    ) -> Result<(), state::IngressParseError> {
        let class = ingress
            .metadata
            .annotations
            .get("kubernetes.io/ingress.class");
        if class.is_none() || !class.unwrap().eq("aether") {
            debug!("Received update from unrelated ingress, ignoring");
            return Ok(());
        }

        *self.ingress.write().await = None;

        debug!("Ingress configuration `{:?}` deleted.", class);

        Ok(())
    }

    async fn set_secret(&self, secret: Secret) -> Result<(), state::SecretParseError> {
        let type_matches = match secret.type_ {
            Some(t) => t.as_str().eq("kubernetes.io/tls"),
            None => false,
        };

        if !type_matches {
            debug!(
                "Received update from unrelated secret type for {:?}.",
                secret.metadata.name
            );
            return Ok(());
        }

        let hosts = match secret.metadata.annotations.get("aether.rs/hosts") {
            Some(hosts) => hosts.split(","),
            None => {
                debug!(
                    "TLS secret {:?} does not have an hosts annotation.",
                    secret.metadata.name
                );
                return Ok(());
            }
        };

        let mut certificates = self.certificates.write().await;

        let crt = secret
            .data
            .get("tls.crt")
            .ok_or(state::SecretParseError::NoSecretData)?;
        let rd: &mut std::io::BufReader<&[u8]> = &mut BufReader::new(crt.0.as_slice());
        let certificate = certs(rd).or(Err(state::SecretParseError::InvalidCertificate))?;

        let key = secret
            .data
            .get("tls.key")
            .ok_or(state::SecretParseError::NoSecretData)?;
        let rd: &mut std::io::BufReader<&[u8]> = &mut BufReader::new(key.0.as_slice());
        let mut pkcs_key =
            rsa_private_keys(rd).or(Err(state::SecretParseError::InvalidPrivateKey))?;
        let signing_key = RSASigningKey::new(&pkcs_key.remove(0)).unwrap();

        let cert = CertifiedKey::new(certificate, Arc::new(Box::new(signing_key)));

        for host in hosts {
            info!("Editted host {}.", host);
            certificates.insert(host.to_string(), cert.clone());
        }

        Ok(())
    }

    async fn remove_secret(&self, secret: Secret) -> Result<(), state::SecretParseError> {
        let type_matches = match secret.type_ {
            Some(t) => t.as_str().eq("kubernetes.io/tls"),
            None => false,
        };

        if !type_matches {
            debug!(
                "Received update from unrelated secret type for {:?}.",
                secret.metadata.name
            );
            return Ok(());
        }

        let hosts = match secret.metadata.annotations.get("aether.rs/hosts") {
            Some(hosts) => hosts.split(","),
            None => {
                debug!(
                    "TLS secret {:?} does not have an hosts annotation.",
                    secret.metadata.name
                );
                return Ok(());
            }
        };

        let mut certificates = self.certificates.write().await;

        for host in hosts {
            info!("Removed host {}.", host);
            certificates.remove(&host.to_string());
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum AuthorityError {
    #[error("There is currently no ingress available.")]
    NoIngress,
    #[error("There is no backend which matches the request.")]
    NoBackend,
}
