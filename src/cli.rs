use clap::Parser;

#[derive(Clone, Parser)]
#[command(
    name = "NodeCookAgent",
    version = "0.1.0",
    author = "long2ice",
    about = "Agent for NodeCook to run jobs"
)]
pub struct Cli {
    /// IPv4 server address
    #[arg(short = '4', long, env = "NCA_IPV4_SERVER")]
    pub ipv4_server: Option<String>,
    /// IPv6 server address
    #[arg(short = '6', long, env = "NCA_IPV6_SERVER")]
    pub ipv6_server: Option<String>,
    /// Port to listen on
    #[arg(short, long, default_value_t = 4000, env = "NCA_PORT")]
    pub port: u16,
    /// API key comes from nodecook to know this node belongs to you
    #[arg(short, long, env = "NCA_API_KEY")]
    pub api_key: String,
    /// Enable debug mode
    #[arg(short, long, default_value_t = false, env = "NCA_DEBUG")]
    pub debug: bool,
    /// Endpoint for agent to access, default is host ip:port, if you are behind proxy, you should set this to your public address
    #[arg(short, long, env = "NCA_ENDPOINT")]
    pub endpoint: Option<String>,
}
