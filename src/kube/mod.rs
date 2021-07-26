use futures_util::{StreamExt, TryStreamExt};
use k8s_openapi::api::networking;
use kube::api::{Api, ListParams, WatchEvent};
use kube::{Client, Error};
use std::convert::TryFrom;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

mod state;

pub struct Kube {
    client: Client,
    ingress: Arc<RwLock<Option<state::Ingress>>>,
}

impl Kube {
    pub async fn connect() -> Result<Self, Error> {
        let client = Client::try_default().await?;

        Ok(Kube {
            client,
            ingress: Arc::new(RwLock::new(None)),
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
                error!("{}", e);
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
}

#[derive(Error, Debug)]
pub enum AuthorityError {
    #[error("There is currently no ingress available.")]
    NoIngress,
    #[error("There is no backend which matches the request.")]
    NoBackend,
}
