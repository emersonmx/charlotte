use anyhow::Result;
use rcgen::{BasicConstraints, Certificate, CertificateParams, DnType, IsCa, KeyPair};
use std::time::SystemTime;

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
        .push(DnType::CommonName, "Charlotte Proxy CA");
    ca_params.not_before = SystemTime::now().into();
    ca_params.not_after =
        (SystemTime::now() + std::time::Duration::from_secs(365 * 24 * 60 * 60)).into();

    let cert: Certificate = ca_params
        .self_signed(&key_pair)
        .map_err(Error::CertificateSigning)?;

    Ok(cert)
}
