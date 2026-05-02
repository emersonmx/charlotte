use anyhow::Result;
use std::{fs, io::Write, path::Path};

fn main() -> Result<()> {
    let key_pair = gencert::generate_key_pair()?;
    let cert = gencert::generate_certificate(&key_pair)?;

    let certs_path = Path::new("certs");
    fs::create_dir_all(certs_path)?;

    let mut cert_file_der = fs::File::create(certs_path.join("ca.crt"))?;
    cert_file_der.write_all(cert.der())?;

    let mut cert_file_pem = fs::File::create(certs_path.join("ca.pem"))?;
    cert_file_pem.write_all(cert.pem().as_bytes())?;

    let mut keypair_file = fs::File::create(certs_path.join("ca.key"))?;
    keypair_file.write_all(key_pair.serialized_der())?;

    Ok(())
}
