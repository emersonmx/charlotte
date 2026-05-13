use rcgen::{BasicConstraints, Certificate, CertificateParams, DnType, IsCa, KeyPair};
use std::{path::Path, time::SystemTime};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to generate key pair: {0}")]
    KeyPairGeneration(#[source] rcgen::Error),
    #[error("Failed to create certificate parameters: {0}")]
    CertificateParamsCreation(#[source] rcgen::Error),
    #[error("Failed to sign certificate: {0}")]
    CertificateSigning(#[source] rcgen::Error),
}

pub fn generate_key_pair() -> Result<KeyPair, Error> {
    KeyPair::generate().map_err(Error::KeyPairGeneration)
}

pub fn generate_certificate(key_pair: &KeyPair) -> Result<Certificate, Error> {
    let mut ca_params = CertificateParams::new(vec![]).map_err(Error::CertificateParamsCreation)?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "Charlene Proxy CA");
    ca_params.not_before = SystemTime::now().into();
    ca_params.not_after =
        (SystemTime::now() + std::time::Duration::from_secs(365 * 24 * 60 * 60)).into();

    let cert: Certificate = ca_params
        .self_signed(&key_pair)
        .map_err(Error::CertificateSigning)?;

    Ok(cert)
}

pub struct CertificateStore {
    ca_cert_bytes: Vec<u8>,
    ca_key_bytes: Vec<u8>,
}

impl CertificateStore {
    pub fn new(ca_cert_bytes: &[u8], ca_key_bytes: &[u8]) -> Result<Self, BoxError> {
        Ok(Self {
            ca_cert_bytes: ca_cert_bytes.to_vec(),
            ca_key_bytes: ca_key_bytes.to_vec(),
        })
    }

    pub async fn from_files(ca_cert_path: &Path, ca_key_path: &Path) -> Result<Self, BoxError> {
        let ca_cert_bytes = tokio::fs::read(ca_cert_path).await.map_err(|e| {
            format!(
                "Failed to read CA certificate from {:?}: {e}",
                &ca_cert_path
            )
        })?;
        let ca_key_bytes = tokio::fs::read(ca_key_path)
            .await
            .map_err(|e| format!("Failed to read CA key from {:?}: {e}", &ca_key_path))?;
        Self::new(&ca_cert_bytes, &ca_key_bytes)
    }

    pub fn generate_cert(
        &self,
        domain: &str,
    ) -> Result<(rcgen::Certificate, rcgen::KeyPair), BoxError> {
        let issuer = self.get_issuer()?;

        let mut params = rcgen::CertificateParams::new(vec![domain.to_string()])
            .map_err(|e| format!("Failed to create certificate params: {e}"))?;
        params.is_ca = rcgen::IsCa::NoCa;
        let domain_keypair =
            rcgen::KeyPair::generate().map_err(|e| format!("Failed to generate key pair: {e}"))?;
        let domain_cert = params
            .signed_by(&domain_keypair, &issuer)
            .map_err(|e| format!("Failed to sign certificate: {e}"))?;

        Ok((domain_cert, domain_keypair))
    }

    fn get_issuer(&self) -> Result<rcgen::Issuer<'_, rcgen::KeyPair>, BoxError> {
        let ca_cert_der = rustls::pki_types::CertificateDer::from(self.ca_cert_bytes.clone());
        let ca_key_der = rustls::pki_types::PrivatePkcs8KeyDer::from(self.ca_key_bytes.clone());

        let ca_key_pair = rcgen::KeyPair::try_from(&ca_key_der)
            .map_err(|e| format!("Failed to parse CA key: {e}"))?;
        let issuer = rcgen::Issuer::from_ca_cert_der(&ca_cert_der, ca_key_pair)
            .map_err(|e| format!("Failed to create issuer from CA cert: {e}"))?;

        Ok(issuer)
    }
}
