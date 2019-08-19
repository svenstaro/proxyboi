use actix_web::client::Client;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use futures::Future;
use std::net::SocketAddr;
use structopt::StructOpt;
use url::Url;

#[derive(StructOpt, Clone)]
#[structopt(
    name = "proxyboi",
    raw(global_settings = "&[structopt::clap::AppSettings::ColoredHelp]")
)]
struct Config {
    #[structopt(short, long, default_value = "0.0.0.0:8080")]
    listen: SocketAddr,

    #[structopt(short = "k", long)]
    insecure: bool,

    #[structopt(help = "Upstream proxy to use (eg. http://localhost:8080)")]
    upstream: Url,
}

fn forward(
    req: HttpRequest,
    payload: web::Payload,
    url: web::Data<Url>,
    client: web::Data<Client>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    // Figure out new URL like such:
    // Old URL: http://localhost:8080/foo?bar=1
    // New URL: https://0.0.0.0:8081/foo?bar=1
    // So in effect, we have to change `protocol`, `host`, `port` and keep `path` and `query`.
    let mut new_url = url.get_ref().clone();
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

    // We want to append the current host to the forwarded for list.
    // In order to do this, we have to parse the possibly existing Forwarded and X-Forwarded-For
    // headers and append the current host value those.
    // This is done independently for Forwarded and X-Forwarded-For because even though it would be
    // very odd for them to have different values, it's certainly possible and not technically
    // invalid.

    // The Forwarded header is a bit nasty to parse. It can look like this:
    // Forwarded: by=<by>;for=<foo>;host=<host>;proto=<http|https>
    // but also like this
    // Forwarded: for=<foo>
    // but also this
    // Forwarded: for=<foo>, for=<bar>
    // also finally also this
    // Forwarded: by=<by>;for=<foo>, for=<bar>;host=<host>
    let forwarded = req
        .headers()
        .get("forwarded")
        .map(|x| x.to_str().unwrap_or(""));
    let forwarded_appended = if let Some(forwarded) = forwarded {
        let for_start = forwarded.find("for=");
        if let Some(for_start) = for_start {
            // Try to find a ';' which ends a `for` subfield.
            // If there is none, the string field is the last field in the header.
            let forwarded_for = if let Some(for_end) = forwarded[for_start..].find(';') {
                &forwarded[for_start..for_start + for_end]
            } else {
                &forwarded[for_start..forwarded.len()]
            };

            let forwarded_for_appended = format!("{}, for={}", forwarded_for, peer);

            // Now we have to place this newly appended list of items back into the header.
            forwarded.replace(forwarded_for, &forwarded_for_appended)
        } else {
            // This is for the case in which the is a Forwarded header but it doesn't have a `for`
            // subfield.
            format!("for={}", peer)
        }
    } else {
        // This is for the case where there is no Forwarded header yet at all.
        format!("for={}", peer)
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
        .set_header("forwarded", forwarded_appended)
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-Proto
        .set_header("x-forwarded-proto", protocol)
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-Host
        .set_header("x-forwarded-host", host)
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Forwarded-For
        .set_header("x-forwarded-for", x_forwarded_for_appended);

    forwarded_req
        .send_stream(payload)
        .map_err(Error::from)
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
    let args = Config::from_args();
    let listen = args.listen;

    HttpServer::new(move || {
        App::new()
            .data(Client::new())
            .data(args.upstream.clone())
            .wrap(middleware::Logger::default())
            .default_service(web::route().to_async(forward))
    })
    .bind(listen)?
    .system_exit()
    .run()
}
