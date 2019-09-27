use std::net::SocketAddr;
use std::path::PathBuf;
use structopt::StructOpt;
use url::Url;

fn from_url(s: &str) -> Result<Url, String> {
    let u = s.parse::<Url>().map_err(|e| e.to_string())?;
    if u.scheme() != "http" && u.scheme() != "https" && u.host().is_none() {
        return Err("Invalid protocol! Must be http or https.".to_string());
    }
    Ok(u)
}

#[derive(StructOpt, Clone)]
#[structopt(
    name = "proxyboi",
    author,
    about,
    global_settings = &[structopt::clap::AppSettings::ColoredHelp],
)]
pub struct ProxyboiConfig {
    /// Socket to listen on
    #[structopt(short, long, default_value = "0.0.0.0:8080")]
    pub listen: SocketAddr,

    /// Allow connections against upstream proxies with invalid TLS certificates
    #[structopt(short = "k", long)]
    pub insecure: bool,

    /// Be quiet (log nothing)
    #[structopt(short, long)]
    pub quiet: bool,

    /// Be verbose (log data of incoming and outgoing requests)
    #[structopt(short, long)]
    pub verbose: bool,

    /// Upstream proxy to use (eg. http://localhost:8080)
    #[structopt(parse(try_from_str = from_url))]
    pub upstream: Url,

    /// TLS cert to use
    #[structopt(long = "cert", requires = "tls-key")]
    pub tls_cert: Option<PathBuf>,

    /// TLS key to use
    #[structopt(long = "key", requires = "tls-cert")]
    pub tls_key: Option<PathBuf>,
}
