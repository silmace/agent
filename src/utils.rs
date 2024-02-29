use crate::cli::Cli;
use crate::constant::{V4_SERVER, V6_SERVER};
use crate::{api::add_agent, constant::VERSION};
use tracing::error;

pub async fn add_agent_with_args(args: Cli, ip_type: &str, init: bool) -> bool {
    match ip_type {
        "ipv4" => {
            add_agent(
                args.ipv4_server.unwrap_or(V4_SERVER.to_string()),
                args.port,
                args.api_key,
                args.endpoint,
                VERSION,
                "ipv4",
                init,
            )
            .await
        }
        "ipv6" => {
            add_agent(
                args.ipv6_server.unwrap_or(V6_SERVER.to_string()),
                args.port,
                args.api_key,
                args.endpoint,
                VERSION,
                "ipv6",
                init,
            )
            .await
        }
        _ => {
            error!("ip_type must be ipv4 or ipv6");
            false
        }
    }
}

pub fn is_ip(ip: &str) -> bool {
    ip.parse::<std::net::IpAddr>().is_ok()
}
