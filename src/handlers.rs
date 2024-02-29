use crate::dns;
use crate::errors::SocketIOError;
use crate::utils::is_ip;
use rand::random;
use serde_json::{json, Value};
use socketioxide::extract::Bin;
use socketioxide::extract::Data;
use socketioxide::extract::SocketRef;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;
use surge_ping::{Client, Config, IcmpPacket, PingIdentifier, PingSequence, ICMP};
use tokio::net;
use tokio::sync::mpsc;
use tokio::time;
use tracert::trace::Tracer;
use tracing::debug;
use tracing::error;
use url::Url;

pub async fn ping(socket: SocketRef, data: Value) {
    let (tx, mut rx) = mpsc::channel::<bool>(1);
    socket.on(
        "disconnect",
        |_socket: SocketRef, Data::<Value>(_data), Bin(_bin)| async move {
            tx.send(true).await.unwrap();
            debug!("ping disconnect by client");
        },
    );
    debug!("receive ping request: {}", data);
    let host = data["host"].as_str().unwrap();
    let single = data["single"].as_bool().unwrap_or(true);
    let is_ipv4 = data["is_ipv4"].as_bool().unwrap_or(true);
    let ns: Option<&str> = data["ns"].as_str();
    let record_type: &str = if is_ipv4 { "A" } else { "AAAA" };
    let ip = if is_ip(host) {
        host.to_string()
    } else {
        let res = dns::resolve(host, record_type, ns).await;
        if res.is_none() {
            socket
                .emit(
                    "ping",
                    json!({
                        "error":SocketIOError::ErrDNSLookupFailed
                    }),
                )
                .unwrap();
            return;
        }
        res.unwrap()
            .iter()
            .filter(|ip| is_ip(&ip.to_string()))
            .next()
            .unwrap()
            .to_string()
    };
    let mut config_builder = Config::builder();
    if is_ipv4 {
        config_builder = config_builder.kind(ICMP::V4);
    } else {
        config_builder = config_builder.kind(ICMP::V6);
    }
    let config = config_builder.build();
    let client = Client::new(&config).unwrap();
    let payload = [0; 56];
    let mut pinger = client
        .pinger(ip.parse().unwrap(), PingIdentifier(random()))
        .await;
    let mut interval = time::interval(Duration::from_secs(1));
    pinger.timeout(Duration::from_secs(1));
    let times = if single { 1 } else { 100 };
    for idx in 0..times {
        if rx.try_recv().is_ok() {
            return;
        }
        interval.tick().await;
        match pinger.ping(PingSequence(idx), &payload).await {
            Ok((IcmpPacket::V4(packet), dur)) => socket
                .emit(
                    "ping",
                    json!({
                        "ip": packet.get_source(),
                        "duration": Some(dur).map(|d| d.as_millis()),
                        "seq": packet.get_sequence().0+1
                    }),
                )
                .unwrap(),
            Ok((IcmpPacket::V6(packet), dur)) => socket
                .emit(
                    "ping",
                    json!({
                        "ip": packet.get_source(),
                        "duration": Some(dur).map(|d| d.as_millis()),
                        "seq": packet.get_sequence().0+1
                    }),
                )
                .unwrap(),
            Err(e) => {
                error!("ping {} failed: {}", host, e);
                socket
                    .emit(
                        "ping",
                        json!({
                            "ip": ip,
                            "duration": None::<u64>,
                            "seq": idx+1,
                            "error": SocketIOError::ErrPingFailed,
                        }),
                    )
                    .unwrap()
            }
        }
    }
    socket.disconnect().unwrap();
}

pub async fn tcping(socket: SocketRef, data: Value) {
    let (tx, mut rx) = mpsc::channel::<bool>(1);
    socket.on(
        "disconnect",
        |_socket: SocketRef, Data::<Value>(_data)| async move {
            tx.send(true).await.unwrap();
            debug!("tcping disconnect by client");
        },
    );
    debug!("receive tcping request: {}", data);
    let host = data["host"].as_str().unwrap();
    let domain = host.split(":").next().unwrap();
    let single = data["single"].as_bool().unwrap_or(true);
    let is_ipv4 = data["is_ipv4"].as_bool().unwrap_or(true);
    let ns: Option<&str> = data["ns"].as_str();
    let record_type = if is_ipv4 { "A" } else { "AAAA" };
    let ip = if is_ip(domain) {
        domain.to_string()
    } else {
        let res = dns::resolve(domain, record_type, ns).await;
        if res.is_none() {
            socket
                .emit(
                    "tcping",
                    json!({
                        "error": SocketIOError::ErrDNSLookupFailed
                    }),
                )
                .unwrap();
            return;
        }
        res.unwrap()
            .iter()
            .filter(|ip| is_ip(&ip.to_string()))
            .next()
            .unwrap()
            .to_string()
    };
    let times = if single { 1 } else { 100 };
    let mut interval = time::interval(Duration::from_secs(1));
    for idx in 0..times {
        if rx.try_recv().is_ok() {
            return;
        }
        interval.tick().await;
        let start = std::time::Instant::now();
        let res = net::TcpStream::connect(host).await;
        match res {
            Ok(_) => {
                let ms = start.elapsed().as_millis();
                socket
                    .emit(
                        "tcping",
                        json!({
                            "ip": ip,
                            "duration": ms,
                            "seq": idx+1
                        }),
                    )
                    .unwrap();
            }
            Err(e) => {
                error!("tcping {} failed: {}", ip.to_string(), e);
                socket
                    .emit(
                        "tcping",
                        json!({
                            "ip": ip,
                            "seq": idx+1,
                            "error": SocketIOError::ErrTCPingFailed,
                        }),
                    )
                    .unwrap();
            }
        };
    }
    socket.disconnect().unwrap();
}

pub async fn dns(socket: SocketRef, data: Value) {
    debug!("receive dns request: {}", data);
    let domain = data["domain"].as_str().unwrap();
    let type_ = data["type"].as_str().unwrap();
    let ns = data["ns"].as_str();
    let start = std::time::Instant::now();
    let res = dns::resolve(domain, type_, ns).await;
    match res {
        Some(res) => {
            let ms = start.elapsed().as_millis();
            let ips = if type_ == "CNAME" {
                res.iter().map(|ip| ip.to_string()).collect::<Vec<String>>()
            } else {
                res.iter()
                    .filter(|ip| is_ip(&ip.to_string()))
                    .map(|ip| ip.to_string())
                    .collect::<Vec<String>>()
            };
            socket
                .emit(
                    "dns",
                    json!({
                        "duration": ms,
                        "ips": ips,
                    }),
                )
                .unwrap();
        }
        None => {
            socket
                .emit(
                    "dns",
                    json!({
                        "error": SocketIOError::ErrDNSLookupFailed
                    }),
                )
                .unwrap();
        }
    }
}

pub async fn mtr(socket: SocketRef, data: Value) {
    debug!("receive mtr request: {}", data);
    let host = data["host"].as_str().unwrap();
    let ns = data["ns"].as_str();
    let is_ipv4 = data["is_ipv4"].as_bool().unwrap_or(true);
    let record_type = if is_ipv4 { "A" } else { "AAAA" };
    let ip = if is_ip(host) {
        host.to_string()
    } else {
        let res = dns::resolve(host, record_type, ns).await;
        if res.is_none() {
            socket
                .emit(
                    "mtr",
                    json!({
                        "error": SocketIOError::ErrDNSLookupFailed
                    }),
                )
                .unwrap();
            return;
        }
        res.unwrap()
            .iter()
            .filter(|ip| is_ip(&ip.to_string()))
            .next()
            .unwrap()
            .to_string()
    };
    let tracer: Tracer = Tracer::new(ip.parse().unwrap()).unwrap();
    let rx = tracer.get_progress_receiver();
    let handle = thread::spawn(move || tracer.trace());
    let mut sent_hops = Vec::new();
    while let Ok(msg) = rx.lock().unwrap().recv() {
        sent_hops.push(msg.hop);
        socket
            .emit(
                "mtr",
                json!({
                    "seq": msg.seq,
                    "ip_addr": msg.ip_addr,
                    "host_name": msg.host_name,
                    "ttl": msg.ttl,
                    "hop": msg.hop,
                    "node_type": match msg.node_type {
                        tracert::node::NodeType::DefaultGateway=> "DefaultGateway",
                        tracert::node::NodeType::Relay=> "Relay",
                        tracert::node::NodeType::Destination=> "Destination",
                    },
                    "rtt": msg.rtt.as_millis(),
                }),
            )
            .unwrap();
    }
    match handle.join().unwrap() {
        Ok(r) => {
            for node in r.nodes {
                if sent_hops.contains(&node.hop) {
                    continue;
                }
                socket
                    .emit(
                        "mtr",
                        json!({
                            "seq": node.seq,
                            "ip_addr": node.ip_addr,
                            "host_name": node.host_name,
                            "ttl": node.ttl,
                            "hop": node.hop,
                            "node_type": match node.node_type {
                                tracert::node::NodeType::DefaultGateway=> "DefaultGateway",
                                tracert::node::NodeType::Relay=> "Relay",
                                tracert::node::NodeType::Destination=> "Destination",
                            },
                            "rtt": node.rtt.as_millis(),
                        }),
                    )
                    .unwrap();
            }
        }
        Err(e) => {
            error!("mtr {} failed: {}", host, e);
            socket
                .emit(
                    "mtr",
                    json!({
                        "error": SocketIOError::ErrMTRFailed
                    }),
                )
                .unwrap();
        }
    }
}

pub async fn http(socket: SocketRef, data: Value) {
    debug!("receive http request: {}", data);
    let url = data["url"].as_str().unwrap();
    let ns = data["ns"].as_str();
    let is_ipv4 = data["is_ipv4"].as_bool().unwrap_or(true);
    let record_type = if is_ipv4 { "A" } else { "AAAA" };
    let parsed_url = Url::parse(url).unwrap();
    let host = parsed_url.host_str().unwrap();
    let port = parsed_url.port().unwrap_or(80);
    let start = std::time::Instant::now();
    let ip = if is_ip(host) {
        host.to_string()
    } else {
        let res = dns::resolve(host, record_type, ns).await;
        if res.is_none() {
            socket
                .emit(
                    "http",
                    json!({
                        "error": SocketIOError::ErrDNSLookupFailed
                    }),
                )
                .unwrap();
            return;
        }
        res.unwrap()
            .iter()
            .filter(|ip| is_ip(&ip.to_string()))
            .next()
            .unwrap()
            .to_string()
    };
    let dns_duration = start.elapsed().as_millis();
    let client: reqwest::Client;
    if is_ipv4 {
        let localhost_v4 = IpAddr::V4("0.0.0.0".parse().unwrap());
        client = reqwest::Client::builder()
            .local_address(localhost_v4)
            .resolve(host, SocketAddr::new(ip.parse().unwrap(), port))
            .build()
            .unwrap();
    } else {
        let localhost_v6 = IpAddr::V6("::".parse().unwrap());
        client = reqwest::Client::builder()
            .local_address(localhost_v6)
            .resolve(host, SocketAddr::new(ip.parse().unwrap(), port))
            .build()
            .unwrap();
    }
    let res = client.get(url).send().await;
    match res {
        Ok(res) => {
            let status = res.status().as_u16();
            socket
                .emit(
                    "http",
                    json!({
                        "duration": start.elapsed().as_millis(),
                        "ip": ip,
                        "dns_duration": dns_duration,
                        "status": status,
                    }),
                )
                .unwrap();
        }
        Err(e) => {
            error!("http {} failed: {}", url, e);
            socket
                .emit(
                    "http",
                    json!({
                        "duration": start.elapsed().as_millis(),
                        "dns_duration": dns_duration,
                        "ip": ip,
                        "error": SocketIOError::ErrHTTPFailed,
                    }),
                )
                .unwrap();
        }
    }
}
