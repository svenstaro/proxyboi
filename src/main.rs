mod config;
mod error;
mod forwarded_header;
mod handler;
mod logging;
mod tls_utils;

use std::sync::Arc;
use std::time::Duration;

use actix_web::client::{ClientBuilder, Connector};
use actix_web::{web, App, HttpServer};
use clap::Parser;
use log::trace;
use rustls::{
    Certificate, ClientConfig, NoClientAuth, RootCertStore, ServerCertVerified, ServerCertVerifier,
    ServerConfig, TLSError,
};

use crate::config::ProxyboiConfig;
use crate::tls_utils::load_cert;
use crate::tls_utils::load_private_key;

struct NoVerifier;

impl ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _roots: &RootCertStore,
        _presented_certs: &[Certificate],
        dns_name: webpki::DNSNameRef,
        _ocsp_response: &[u8],
    ) -> Result<ServerCertVerified, TLSError> {
        trace!("decoding dns: {:#?}", dns_name);
        Ok(ServerCertVerified::assertion())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    #[cfg(windows)]
    use yansi::Paint;
    #[cfg(windows)]
    Paint::enable_windows_ascii();

    let config = ProxyboiConfig::parse();

    let log_level = if config.quiet {
        simplelog::LevelFilter::Error
    } else {
        simplelog::LevelFilter::Info
    };

    if simplelog::TermLogger::init(
        log_level,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .is_err()
    {
        simplelog::SimpleLogger::init(log_level, simplelog::Config::default())
            .expect("Couldn't initialize logger")
    }

    let config_ = config.clone();
    let mut http_server = HttpServer::new(move || {
        let connector = if config.insecure {
            let mut client_config = ClientConfig::new();
            client_config
                .dangerous()
                .set_certificate_verifier(Arc::new(NoVerifier {}));
            Connector::new()
                .rustls(Arc::new(client_config))
                .timeout(Duration::from_secs(config.timeout))
                .finish()
        } else {
            Connector::new()
                .timeout(Duration::from_secs(config.timeout))
                .finish()
        };
        let client = ClientBuilder::new().connector(connector).finish();

        App::new()
            .data(client)
            .data(config.clone())
            .default_service(web::route().to(handler::forward))
    });

    if let (Some(tls_cert), Some(tls_key)) = (config_.tls_cert, config_.tls_key) {
        let cert_file = load_cert(&tls_cert)?;
        let key_file = load_private_key(&tls_key)?;
        let mut rustls_config = ServerConfig::new(NoClientAuth::new());
        rustls_config
            .set_single_cert(cert_file, key_file)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        http_server = http_server.bind_rustls(config_.listen, rustls_config)?;
    } else {
        http_server = http_server.bind(config_.listen)?;
    }
    http_server.run().await
}
