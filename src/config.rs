use actix_web::http::{HeaderMap, HeaderName, HeaderValue};
use clap::Clap;
use std::net::SocketAddr;
use std::path::PathBuf;
use url::Url;

/// Parse a header given in a string format into a `HeaderMap`
///
/// Headers are expected to be in format "key:value".
fn parse_header(header: &str) -> Result<HeaderMap, String> {
    let header: Vec<&str> = header.split(':').collect();
    if header.len() != 2 {
        return Err("Wrong header format (see --help for format)".to_string());
    }

    let (header_name, header_value) = (header[0], header[1]);

    let hn = HeaderName::from_lowercase(header_name.to_lowercase().as_bytes())
        .map_err(|e| e.to_string())?;

    let hv = HeaderValue::from_str(header_value).map_err(|e| e.to_string())?;

    let mut map = HeaderMap::new();
    map.insert(hn, hv);
    Ok(map)
}

#[derive(Clap, Clone)]
#[clap(
    name = "proxyboi",
    author,
    about,
    setting = clap::AppSettings::ColoredHelp,
)]
pub struct ProxyboiConfig {
    /// Socket to listen on
    #[clap(short, long, default_value = "0.0.0.0:8080")]
    pub listen: SocketAddr,

    /// Allow connections against upstream proxies with invalid TLS certificates
    #[clap(short = 'k', long)]
    pub insecure: bool,

    /// Be quiet (log nothing)
    #[clap(short, long)]
    pub quiet: bool,

    /// Be verbose (log data of incoming and outgoing requests)
    #[clap(short, long)]
    pub verbose: bool,

    /// Upstream server to proxy to (eg. http://localhost:8080)
    #[clap()]
    pub upstream: Url,

    /// Additional headers to send to upstream server
    #[clap(long, parse(try_from_str = parse_header))]
    pub upstream_header: Vec<HeaderMap>,

    /// Additional response headers to send to requesting client
    #[clap(long, parse(try_from_str = parse_header))]
    pub response_header: Vec<HeaderMap>,

    /// Connection timeout against upstream in seconds (including DNS name resolution)
    #[clap(long, default_value = "5")]
    pub timeout: u64,

    /// TLS cert to use
    #[clap(long = "cert", requires = "tls-key")]
    pub tls_cert: Option<PathBuf>,

    /// TLS key to use
    #[clap(long = "key", requires = "tls-cert")]
    pub tls_key: Option<PathBuf>,
}
