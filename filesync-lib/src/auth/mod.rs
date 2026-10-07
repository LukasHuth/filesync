use anyhow::Result;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair, KeyUsagePurpose, SanType,
};
use std::{net::IpAddr, path::Path};
use time::{Duration, OffsetDateTime};

use crate::vpn::VPNConfig;

pub struct CA {
    issuer: Issuer<'static, KeyPair>,
}

impl CA {
    pub fn setup_with_config(config: VPNConfig) -> Result<()> {
        CA::setup(
            &config.pki_dir,
            &config.tls_server_name,
            config.server_ip.into(),
        )
    }
    pub fn setup(dir: &Path, server_name: &str, server_ip: IpAddr) -> Result<()> {
        let ca = Self::load_or_create(dir)?;

        if !dir.join("client.pem").exists() {
            let (c, k) = ca.issue_as_string("my-client", None, false)?;
            std::fs::write(dir.join("client.pem"), c)?;
            write_private(&dir.join("client.key"), &k)?;
        }
        if !dir.join("server.pem").exists() {
            let (c, k) = ca.issue_as_string(server_name, Some(server_ip), true)?;
            std::fs::write(dir.join("server.pem"), c)?;
            write_private(&dir.join("server.key"), &k)?;
        }
        Ok(())
    }
    /// Load the CA from `dir`, or create and persist it on first run.
    pub fn load_or_create(dir: &Path) -> Result<Self> {
        let cert_path = dir.join("ca.pem");
        let key_path = dir.join("ca.key");
        if cert_path.exists() && key_path.exists() {
            let key = KeyPair::from_pem(&std::fs::read_to_string(&key_path)?)?;
            let issuer = Issuer::from_ca_cert_pem(&std::fs::read_to_string(&cert_path)?, key)?;
            return Ok(Self { issuer });
        }
        std::fs::create_dir_all(dir)?;
        let key = KeyPair::generate()?;
        let mut params = CertificateParams::new(Vec::<String>::new())?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Filesync CA");
        params.distinguished_name = dn;
        params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0)); // may sign leaves only
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::hours(1);
        params.not_after = now + Duration::days(3650);
        let cert = params.self_signed(&key)?;
        std::fs::write(&cert_path, cert.pem())?;
        write_private(&key_path, &key.serialize_pem())?;

        let issuer = Issuer::new(params, key);
        Ok(Self { issuer })
    }
    /// Issue a leaf cert. Returns (cert_pem, key_pem)
    pub fn issue_as_string(
        &self,
        name: &str,
        ip: Option<std::net::IpAddr>,
        server: bool,
    ) -> Result<(String, String)> {
        self.issue(name, ip, server)
            .map(|(cert, leaf_key)| (cert.pem(), leaf_key.serialize_pem()))
    }
    /// Issue a leaf cert. Returns (cert, key)
    pub fn issue(
        &self,
        name: &str,
        ip: Option<std::net::IpAddr>,
        server: bool,
    ) -> Result<(Certificate, KeyPair)> {
        let leaf_key = KeyPair::generate()?;
        let mut params = CertificateParams::new(vec![name.to_string()])?;
        if let Some(ip) = ip {
            params.subject_alt_names.push(SanType::IpAddress(ip));
        }
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, name);
        params.distinguished_name = dn;
        params.is_ca = IsCa::NoCa;
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![if server {
            ExtendedKeyUsagePurpose::ServerAuth
        } else {
            ExtendedKeyUsagePurpose::ClientAuth
        }];
        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::hours(1);
        params.not_after = now + Duration::days(365);
        let cert = params.signed_by(&leaf_key, &self.issuer)?;

        Ok((cert, leaf_key))
    }
}
fn write_private(path: &Path, contents: &str) -> Result<()> {
    use std::io::Write;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(contents.as_bytes())?;
    }
    #[cfg(not(unix))]
    fs::write(path, contents)?;
    Ok(())
}
