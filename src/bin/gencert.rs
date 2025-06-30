use rcgen::{CertifiedKey, generate_simple_self_signed};

fn main() {
    let subject_alt_names = vec!["localhost".to_string()];

    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names).unwrap();
    println!("{}", cert.pem());
    println!("{}", key_pair.serialize_pem());
}
