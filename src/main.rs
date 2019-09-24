use actix_web::client::Client;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use futures::Future;
use rustls::{NoClientAuth, ServerConfig};
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;
use structopt::StructOpt;

mod args;
mod tls_utils;
mod utils;

use crate::args::ProxyboiConfig;
use crate::tls_utils::{load_cert, load_private_key};
use crate::utils::ForwardedHeader;

fn forward(
    req: HttpRequest,
    payload: web::Payload,
    args: web::Data<ProxyboiConfig>,
    client: web::Data<Client>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    // Figure out new URL like such:
    // Old URL: http://localhost:8080/foo?bar=1
    // New URL: https://0.0.0.0:8081/foo?bar=1
    // So in effect, we have to change `protocol`, `host`, `port` and keep `path` and `query`.
    let mut new_url = args.upstream.clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    let protocol = req.uri().scheme_str().unwrap_or("http");
    let host = req
        .headers()
        .get("host")
        .map(|x| x.to_str().unwrap_or("unknown"))
        .unwrap_or("unknown");
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
        format!("{}, HTTP/1.1 proxyboi", via)
    } else {
        format!("HTTP/1.1 proxyboi")
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

    let forwarded_req = client
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

    forwarded_req
        .send_stream(payload)
        .map_err(actix_web::Error::from)
        .map(|res| {
            let mut client_resp = HttpResponse::build(res.status());
            for (header_name, header_value) in res
                .headers()
                .iter()
                .filter(|(h, _)| *h != "connection" && *h != "content-length")
            {
                client_resp.header(header_name.clone(), header_value.clone());
            }
            client_resp.streaming(res)
        })
}

fn main() -> std::io::Result<()> {
    let args = ProxyboiConfig::from_args();

    let args_ = args.clone();
    let mut server = HttpServer::new(move || {
        App::new()
            .data(Client::new())
            .data(args_.clone())
            .wrap(middleware::Logger::default())
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
