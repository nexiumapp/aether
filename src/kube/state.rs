use k8s_openapi::api::networking;
use kube::{error::ErrorResponse, ResourceExt};
use std::convert::{TryFrom, TryInto};
use thiserror::Error;

#[derive(Debug)]
pub struct Ingress {
    pub namespace: String,
    pub default_route: Option<IngressBackend>,
    pub rules: Vec<IngressRule>,
}

impl TryFrom<networking::v1::Ingress> for Ingress {
    type Error = IngressParseError;

    fn try_from(api_ingress: networking::v1::Ingress) -> Result<Self, Self::Error> {
        let namespace = api_ingress
            .namespace()
            .ok_or(IngressParseError::MissingNamespace)?;

        let api_spec = api_ingress.spec.ok_or(Self::Error::MissingSpec)?;

        let default_route = match api_spec.default_backend {
            Some(backend) => Some(TryFrom::try_from(backend)?),
            None => None,
        };

        let mut rules = Vec::with_capacity(api_spec.rules.len());

        for rule in api_spec.rules {
            rules.push(TryInto::try_into(rule)?);
        }

        Ok(Ingress {
            namespace,
            default_route,
            rules,
        })
    }
}

#[derive(Debug)]
pub struct IngressRule {
    pub host: Option<String>,
    pub paths: Vec<IngressPath>,
}

impl TryFrom<networking::v1::IngressRule> for IngressRule {
    type Error = IngressParseError;

    fn try_from(api_rule: networking::v1::IngressRule) -> Result<Self, Self::Error> {
        let http = api_rule.http.ok_or(IngressParseError::MissingHttpRule)?;

        let mut paths = Vec::with_capacity(http.paths.len());

        for path in http.paths {
            paths.push(TryFrom::try_from(path)?);
        }

        Ok(IngressRule {
            host: api_rule.host,
            paths,
        })
    }
}

#[derive(Debug)]
pub struct IngressPath {
    pub path: String,
    pub backend: IngressBackend,
}

impl TryFrom<networking::v1::HTTPIngressPath> for IngressPath {
    type Error = IngressParseError;

    fn try_from(api_path: networking::v1::HTTPIngressPath) -> Result<Self, Self::Error> {
        let path = api_path.path.ok_or(IngressParseError::MissingPath)?;
        let backend = api_path.backend.try_into()?;

        Ok(IngressPath { path, backend })
    }
}

#[derive(Debug)]
pub struct IngressBackend {
    pub name: String,
    pub port: u32,
}

impl TryFrom<networking::v1::IngressBackend> for IngressBackend {
    type Error = IngressParseError;

    fn try_from(api_backend: networking::v1::IngressBackend) -> Result<Self, Self::Error> {
        let api_service = api_backend
            .service
            .ok_or(IngressParseError::NoBackendService)?;
        let api_port = api_service.port.ok_or(IngressParseError::NoServicePort)?;

        let name = api_service.name;
        let port = api_port.number.ok_or(IngressParseError::NoServicePort)? as u32;

        Ok(IngressBackend { name, port })
    }
}

#[derive(Error, Debug)]
pub enum IngressParseError {
    #[error("Watch error: {0}")]
    WatchError(ErrorResponse),
    #[error("Ingress is missing the namespace.")]
    MissingNamespace,
    #[error("Ingress specifications are missing.")]
    MissingSpec,
    #[error("Backend does not have a service defined.")]
    NoBackendService,
    #[error("Service does not have a port defined.")]
    NoServicePort,
    #[error("Ingress rule does not provide an http rule.")]
    MissingHttpRule,
    #[error("Ingress rule misses the path.")]
    MissingPath,
}

#[derive(Error, Debug)]
pub enum SecretParseError {
    #[error("Watch error: {0}")]
    WatchError(ErrorResponse),
    #[error("The secret did not contain the certificate/key.")]
    NoSecretData,
    #[error("The private key was not in PKCS/8 format.")]
    InvalidPrivateKey,
    #[error("The certificate was in an invalid format.")]
    InvalidCertificate,
}
