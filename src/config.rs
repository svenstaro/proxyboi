use clap::Clap;
use std::net::SocketAddr;
use std::path::PathBuf;
use url::Url;

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

    /// Upstream proxy to use (eg. http://localhost:8080)
    #[clap()]
    pub upstream: Url,

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
