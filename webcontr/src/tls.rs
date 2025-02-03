use std::{fs::File, io::Read};

use tokio_rustls::rustls::{
  pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
  ServerConfig,
};

#[derive(Clone)]
pub struct TLSPaths {
  cert_path: String,
  key_path: String,
}

impl TLSPaths {
  pub fn from_paths(
    cert_path: impl Into<String>,
    key_path: impl Into<String>,
  ) -> Self {
    Self { cert_path: cert_path.into(), key_path: key_path.into() }
  }
}

impl TLSPaths {
  pub fn serverconfig_from_paths(self) -> ServerConfig {
    let mut file = File::open(self.cert_path).unwrap();
    let mut content = Vec::default();
    file.read_to_end(&mut content).unwrap();
    let cert = CertificateDer::pem_slice_iter(&content)
      .collect::<Result<Vec<_>, _>>()
      .unwrap()
      .clone();

    let mut file = File::open(self.key_path).unwrap();
    let mut content = Vec::default();
    file.read_to_end(&mut content).unwrap();
    let key = PrivateKeyDer::from_pem_slice(&content).unwrap();

    ServerConfig::builder()
      .with_no_client_auth()
      .with_single_cert(cert, key)
      .unwrap()
  }
}
