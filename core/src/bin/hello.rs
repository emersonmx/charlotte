use anyhow::{Result, anyhow};
use rustls::{
    crypto::ring::sign::any_supported_type,
    pki_types::{CertificateDer, PrivateKeyDer},
    sign::CertifiedKey,
};
use std::fs;

fn main() -> Result<()> {
    let cert_content = fs::read("certs/ca.crt")?;
    let cert = CertificateDer::from(cert_content);
    let key_pair_content = fs::read("certs/ca.key")?;
    let key_pair = PrivateKeyDer::try_from(key_pair_content).map_err(|e| anyhow!("{}", e))?;
    let key_pair = any_supported_type(&key_pair)?;
    let ck = CertifiedKey::new(vec![cert.clone()], key_pair.clone());

    dbg!(cert);
    dbg!(key_pair);
    dbg!(ck);
    Ok(())
}
