mod api;
mod app;
mod cli;
mod constant;
mod dns;
mod errors;
mod handlers;
mod utils;
use crate::app::create_app;
use crate::cli::Cli;
use crate::utils::add_agent_with_args;
use clap::Parser;
use tokio::signal;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, Level};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut level = Level::INFO;
    let args = Cli::parse();
    let api_key = args.api_key.clone();
    let port = args.port;
    if args.debug {
        level = Level::DEBUG;
    }
    let collector = fmt().with_max_level(level).finish();
    tracing::subscriber::set_global_default(collector)?;
    if args.ipv4_only && args.ipv6_only {
        panic!("ipv4_only and ipv6_only can't be true at the same time");
    }
    let mut v4_ok = false;
    let mut v6_ok = false;
    if !args.ipv4_only {
        v6_ok = add_agent_with_args(args.clone(), "ipv6", true).await;
    }
    if !args.ipv6_only {
        v4_ok = add_agent_with_args(args.clone(), "ipv4", true).await;
    }
    if !v4_ok && !v6_ok {
        panic!("add ipv4 and ipv6 agent failed, please check your network or try again later");
    }
    let sched = JobScheduler::new().await?;
    sched
        .add(Job::new_async("*/60 * * * * *", move |_uuid, _l| {
            let args = args.clone();
            Box::pin(async move {
                if v4_ok && !args.ipv6_only {
                    add_agent_with_args(args.clone(), "ipv4", false).await;
                }
                if v6_ok && !args.ipv4_only {
                    add_agent_with_args(args, "ipv6", false).await;
                }
            })
        })?)
        .await?;
    sched.start().await?;
    let addr = format!(":::{}", port);
    info!("listening on addr {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, create_app(api_key))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
