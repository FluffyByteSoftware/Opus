//! File:       Opus/Conductor/dev/networking/src/tls.rs
//! Component:  Conductor
//! Author:     Jacob Chacko
//!
//! The server's TLS certificate and key, and the settings rustls builds
//! every connection from.  The TCP side asks for them on every START
//! SERVER.
//!
//! Two PEM files, where `networking.cfg` says (`Content/certs/` by
//! default): the certificate, which is the public half and the one file
//! a client gets and trusts, and the private key, which never leaves this
//! machine and never goes in git.  Whoever has the key can pretend to be
//! us.
//!
//! Conductor doesn't make the pair.  It could, with one more crate, but
//! Jacob makes it once by hand with openssl (the command is in the error
//! below and in README.md), which is one crate fewer in the build.  An
//! elliptic curve key, because signing with one costs a tenth of what an
//! RSA key does, and the server signs on every handshake.
//!
//! Self-signed, which means nobody but us vouches for it.  That's fine,
//! because clients never ask anybody else: they trust the copy they were
//! given and refuse any server that shows them a different one.  So a
//! new certificate locks out every client with the old one.  Read through
//! DiskMan, like every file, which means DiskMan holds the key in memory
//! for as long as Conductor runs.

use std::path::Path;
use std::sync::Arc;

use rustls::ServerConfig;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use conductor_tools::diskman;

use crate::settings::Settings;

/// Reads the certificate and the key and hands back the settings every
/// TLS connection is built from.  An `Err` is a sentence for the admin,
/// and never has any of the key in it.
pub fn server_config(settings: &Settings) -> Result<Arc<ServerConfig>, String> {
    let cert_path = &settings.certificate_file;
    let key_path = &settings.private_key_file;

    // The folder, so openssl has somewhere to write into.  Folders stay
    // with std::fs; only files go through DiskMan.
    if let Some(folder) = cert_path.parent() {
        let _ = std::fs::create_dir_all(folder);
    }

    let cert_pem = read(cert_path, settings)?;
    let key_pem = read(key_path, settings)?;

    // A PEM file can hold a chain of certificates, ours first.  A
    // self-signed one is a chain of one.
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(&cert_pem)
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{} isn't a certificate we can read: {e}.", cert_path.display()))?;
    if certs.is_empty() {
        return Err(format!("{} has no certificate in it.", cert_path.display()));
    }
    // No details on this one.  Whatever is wrong with the file, the reason
    // could quote part of it, and part of a key is still a key.
    let key = PrivateKeyDer::from_pem_slice(&key_pem)
        .map_err(|_| format!("{} isn't a private key we can read.", key_path.display()))?;

    // Rust note: the "safe default" versions are whichever ones the crate
    // was built with.  Cargo.toml leaves TLS 1.2 out, so that's 1.3 only.
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("TLS won't start: {e}."))?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("TLS won't use {} and {}: {e}.", cert_path.display(), key_path.display()))?;
    Ok(Arc::new(config))
}

/// One file's bytes, through DiskMan.  A file that isn't there gets the
/// command that makes the pair, since that's what the admin does next.
fn read(path: &Path, settings: &Settings) -> Result<Vec<u8>, String> {
    match diskman::read(path).wait() {
        Ok(contents) => Ok(contents.to_vec()),
        Err(e) if e.is_not_found() => Err(format!("{} isn't there.  Make the certificate and its key once, \
            from any folder, then STOP SERVER and START SERVER:  {}", path.display(),
            make_pair(&settings.certificate_file, &settings.private_key_file))),
        Err(e) => Err(format!("Couldn't read {}: {e}.", path.display())),
    }
}

/// The one-line command that makes the pair, naming the two files where
/// `networking.cfg` says they go, as full paths.  So it works pasted from
/// any folder: with relative paths, pasted from `Conductor/dev` it made a
/// second `Content` in there, and Conductor found that one first
/// (2026-09-29).  A P-256 key, ten years, and the names a client on this
/// machine would connect by.
pub fn make_pair(cert_path: &Path, key_path: &Path) -> String {
    let folder = |path: &Path| path.parent().map_or_else(|| ".".to_string(), |dir| dir.display().to_string());
    format!("mkdir -p \"{}\" \"{}\" && openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
        -nodes -keyout \"{}\" -out \"{}\" -days 3650 -subj \"/CN=Opus Conductor\" \
        -addext \"subjectAltName=DNS:localhost,IP:127.0.0.1\"",
        folder(key_path), folder(cert_path), key_path.display(), cert_path.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_is_one_line_naming_both_files_in_full() {
        let command = make_pair(Path::new("/opt/Opus/Content/certs/conductor.crt"),
                                Path::new("/opt/Opus/Content/certs/conductor.key"));
        assert!(!command.contains('\n'));
        assert!(command.starts_with("mkdir -p \"/opt/Opus/Content/certs\" "));
        assert!(command.contains("-keyout \"/opt/Opus/Content/certs/conductor.key\""));
        assert!(command.contains("-out \"/opt/Opus/Content/certs/conductor.crt\""));
        assert!(!command.contains(" Content/"));
    }
}
