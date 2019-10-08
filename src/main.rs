use actix_web::client::{Client, ClientBuilder, Connector};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use futures::Future;
use log::{error, info, trace};
use rustls::{
    Certificate, ClientConfig, NoClientAuth, RootCertStore, ServerCertVerified, ServerCertVerifier,
    ServerConfig, TLSError,
};
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;
use std::sync::Arc;
use structopt::StructOpt;

mod args;
mod logging;
mod tls_utils;
mod utils;

use crate::args::ProxyboiConfig;
use crate::logging::{
    log_incoming_request, log_outgoing_response, log_upstream_request, log_upstream_response,
};
use crate::tls_utils::{load_cert, load_private_key};
use crate::utils::ForwardedHeader;

fn forward(
    req: HttpRequest,
    body: web::Bytes,
    args: web::Data<ProxyboiConfig>,
    client: web::Data<Client>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    let incoming_request_log = log_incoming_request(&req, args.verbose);

    // Figure out new URL like such:
    // Old URL: http://localhost:8080/foo?bar=1
    // New URL: https://0.0.0.0:8081/foo?bar=1
    // So in effect, we have to change `protocol`, `host`, `port` and keep `path` and `query`.
    let mut new_url = args.upstream.clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    let conn_info = &req.connection_info().clone();
    let protocol = conn_info.scheme();
    let version = req.version();
    let host = conn_info.host();

    let peer = req
        .head()
        .peer_addr
        .map(|p| p.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let forwarded = req
        .headers()
        .get("forwarded")
        .map(|x| x.to_str().unwrap_or(""))
        .unwrap_or("");

    let forwarded_header = ForwardedHeader::from_info(
        &peer,
        &args.listen.ip().to_string(),
        forwarded,
        host,
        protocol,
    );
    let via = if let Some(via) = req.headers().get("via").map(|x| x.to_str().unwrap_or("")) {
        format!(
            "{previous_via}, {version:?} proxyboi",
            previous_via = via,
            version = version
        )
    } else {
        format!("{version:?} proxyboi", version = version)
    };

    // The X-Forwarded-For header is much simpler to handle :)
    let x_forwarded_for = req
        .headers()
        .get("x-forwarded-for")
        .map(|x| x.to_str().unwrap_or(""));
    let x_forwarded_for_appended = if let Some(x_forwarded_for) = x_forwarded_for {
        format!("{}, {}", x_forwarded_for, peer)
    } else {
        peer.clone()
    };

    let upstream_req = client
        .request_from(new_url.as_str(), req.head())
        .no_decompress()
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Forwarded
        .set_header("forwarded", forwarded_header.to_string())
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-Proto
        .set_header("x-forwarded-proto", protocol)
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-Host
        .set_header("x-forwarded-host", host)
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-For
        .set_header("x-forwarded-for", x_forwarded_for_appended)
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Via
        .set_header("via", via);

    let upstream_request_log = log_upstream_request(&upstream_req, args.verbose);

    upstream_req
        .send_body(body)
        .map_err(|x| {
            error!("{}", x);
            actix_web::Error::from(x)
        })
        .map(move |upstream_resp| {
            let upstream_response_log =
                log_upstream_response(&upstream_resp, new_url.as_str(), args.verbose);

            // We need to build this twice in order to log the final outgoing response.
            // It's super ugly but I don't know any other way since we can't clone this.
            let mut resp_for_logging = HttpResponse::build(upstream_resp.status());
            for (header_name, header_value) in upstream_resp
                .headers()
                .iter()
                // Remove `Connection` as per
                // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
                .filter(|(h, _)| *h != "connection")
            {
                resp_for_logging.header(header_name.clone(), header_value.clone());
            }
            let resp_for_logging = resp_for_logging.finish();
            let outgoing_response_log = log_outgoing_response(
                &resp_for_logging,
                &req.connection_info().remote().unwrap_or("unknown"),
                args.verbose,
            );

            let mut resp = HttpResponse::build(upstream_resp.status());
            for (header_name, header_value) in upstream_resp
                .headers()
                .iter()
                // Remove `Connection` as per
                // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
                .filter(|(h, _)| *h != "connection")
            {
                resp.header(header_name.clone(), header_value.clone());
            }
            info!(
                "{incoming_req}\n{upstream_req}\n{upstream_resp}\n{outgoing_resp}",
                incoming_req = incoming_request_log,
                upstream_req = upstream_request_log,
                upstream_resp = upstream_response_log,
                outgoing_resp = outgoing_response_log
            );
            resp.streaming(upstream_resp)
        })
}

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

fn main() -> std::io::Result<()> {
    #[cfg(windows)]
    use yansi::Paint;
    #[cfg(windows)]
    Paint::enable_windows_ascii();

    let args = ProxyboiConfig::from_args();

    let log_level = if args.quiet {
        simplelog::LevelFilter::Error
    } else {
        simplelog::LevelFilter::Info
    };

    if simplelog::TermLogger::init(
        log_level,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
    )
    .is_err()
    {
        simplelog::SimpleLogger::init(log_level, simplelog::Config::default())
            .expect("Couldn't initialize logger")
    }

    let args_ = args.clone();
    let mut server = HttpServer::new(move || {
        let client = if args_.insecure {
            let mut client_config = ClientConfig::new();
            client_config
                .dangerous()
                .set_certificate_verifier(Arc::new(NoVerifier {}));
            let connector = Connector::new().rustls(Arc::new(client_config)).finish();
            ClientBuilder::new().connector(connector).finish()
        } else {
            Client::new()
        };
        App::new()
            .data(client)
            .data(args_.clone())
            .default_service(web::route().to_async(forward))
    });

    // TODO: This conditional is kinda dirty but it'll have to do until we have stable if let chains.
    if args.tls_cert.is_some() && args.tls_key.is_some() {
        let tls_cert = args.tls_cert.unwrap();
        let tls_key = args.tls_key.unwrap();

        let mut config = ServerConfig::new(NoClientAuth::new());
        let cert_file = load_cert(&tls_cert)?;
        let key_file = load_private_key(&tls_key)?;
        config
            .set_single_cert(cert_file, key_file)
            .map_err(|e| IoError::new(IoErrorKind::Other, e.to_string()))?;
        server = server.bind_rustls(args.listen, config)?;
    } else {
        server = server.bind(args.listen)?;
    }
    server.system_exit().run()
}
