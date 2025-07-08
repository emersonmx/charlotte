use anyhow::Result;
use rcgen::{BasicConstraints, Certificate, CertificateParams, DnType, IsCa, KeyPair};
use std::{fs, io::Write, path::Path, time::SystemTime};

fn main() -> Result<()> {
    let key_pair = KeyPair::generate()?;

    let mut ca_params = CertificateParams::new(vec![])?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "Charlotte Proxy CA");
    ca_params.not_before = SystemTime::now().into();
    ca_params.not_after =
        (SystemTime::now() + std::time::Duration::from_secs(365 * 24 * 60 * 60)).into();

    let cert: Certificate = ca_params.self_signed(&key_pair)?;

    let certs_path = Path::new("certs");
    fs::create_dir_all(certs_path)?;

    let mut cert_file = fs::File::create(certs_path.join("ca.crt"))?;
    cert_file.write_all(cert.der())?;

    let mut keypair_file = fs::File::create(certs_path.join("ca.key"))?;
    keypair_file.write_all(key_pair.serialized_der())?;

    Ok(())
}
