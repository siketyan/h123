use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Seek};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use rustls::{Certificate, PrivateKey};
use tracing::{error, info};

use h123::service::StaticFileService;
use h123::Server;

/// An experimental HTTP server in Rust that supports HTTP/1.1, HTTP/2, and HTTP/3 over QUIC.
#[derive(Parser)]
struct Cli {
    /// Path to a certificate chain file in PEM format.
    #[arg(long)]
    cert_chain_pem: String,

    /// Path to a private key file in PEM format.
    #[arg(long)]
    private_key_pem: String,

    /// Path to the document root.
    #[arg(short, long)]
    document_root: PathBuf,

    /// Socket address to bind to.
    #[arg(short, long)]
    bind_to: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    tokio::select!(
        r = run() => {
            match r {
                Ok(_) => (),
                Err(e) => error!("{}", e),
            }
        },
        _ = sigint.recv() => {},
        _ = sigterm.recv() => {},
    );

    info!("Gracefully shutting down...");

    Ok(())
}

async fn run() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let args = Cli::parse();
    let mut privkey_reader = BufReader::new(File::open(args.private_key_pem)?);
    let rustls_config = &rustls::ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(
            rustls_pemfile::certs(&mut BufReader::new(File::open(args.cert_chain_pem)?))?
                .into_iter()
                .map(Certificate)
                .collect(),
            match rustls_pemfile::pkcs8_private_keys(&mut privkey_reader)?
                .into_iter()
                .map(PrivateKey)
                .next()
            {
                Some(k) => k,
                None => {
                    privkey_reader.rewind()?;
                    rustls_pemfile::rsa_private_keys(&mut privkey_reader)?
                        .into_iter()
                        .map(PrivateKey)
                        .next()
                        .unwrap()
                }
            },
        )?;

    Ok(Server::new(
        rustls_config,
        args.bind_to,
        Arc::new(StaticFileService::new(args.document_root.canonicalize()?)),
    )
    .begin()
    .await?)
}
