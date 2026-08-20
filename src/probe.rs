//! Reachability probes: ICMP ping, then TCP and HTTP fallbacks.

use crate::host::Target;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use surge_ping::{Client, Config, ICMP, PingIdentifier, PingSequence};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

const METHOD_TIMEOUT: Duration = Duration::from_millis(1800);
const OVERALL_TIMEOUT: Duration = Duration::from_millis(2500);

static NEXT_IDENT: AtomicU16 = AtomicU16::new(1);

#[derive(Clone)]
pub struct IcmpClients {
    v4: Option<Arc<Client>>,
    v6: Option<Arc<Client>>,
}

impl IcmpClients {
    pub fn new() -> Self {
        let v4 = Client::new(&Config::builder().kind(ICMP::V4).build())
            .ok()
            .map(Arc::new);
        let v6 = Client::new(&Config::builder().kind(ICMP::V6).build())
            .ok()
            .map(Arc::new);
        Self { v4, v6 }
    }

    fn for_ip(&self, ip: IpAddr) -> Option<Arc<Client>> {
        match ip {
            IpAddr::V4(_) => self.v4.clone(),
            IpAddr::V6(_) => self.v6.clone(),
        }
    }
}

/// Return true if the target answered ICMP, TCP, or HTTP.
pub async fn is_reachable(icmp: &IcmpClients, target: &Target) -> bool {
    let icmp = icmp.clone();
    let target = target.clone();
    match timeout(OVERALL_TIMEOUT, probe_inner(icmp, target)).await {
        Ok(v) => v,
        Err(_) => false,
    }
}

async fn probe_inner(icmp: IcmpClients, target: Target) -> bool {
    let mut set = tokio::task::JoinSet::new();

    let ips = resolve_ips(&target.host).await;

    for ip in ips {
        if let Some(client) = icmp.for_ip(ip) {
            set.spawn(async move { icmp_ping(client, ip).await });
        }
        for port in target.probe_ports() {
            set.spawn(async move { tcp_ip(ip, port).await });
        }
    }

    let tcp_host = target.tcp_host();
    for port in target.probe_ports() {
        let host = tcp_host.clone();
        set.spawn(async move { tcp_host_port(&host, port).await });
    }

    // Application-level HTTP when ping is blocked (common for CDNs).
    if target.port.is_none() || target.port == Some(80) {
        let host = target.host.clone();
        set.spawn(async move { http_head(&host, 80).await });
    }

    while let Some(res) = set.join_next().await {
        if matches!(res, Ok(true)) {
            return true;
        }
    }
    false
}

async fn resolve_ips(host: &str) -> Vec<IpAddr> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return vec![ip];
    }
    let host = host.to_string();
    tokio::task::spawn_blocking(move || {
        (host.as_str(), 0)
            .to_socket_addrs()
            .map(|iter| {
                let mut ips: Vec<IpAddr> = iter.map(|a| a.ip()).collect();
                ips.sort();
                ips.dedup();
                ips
            })
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

async fn icmp_ping(client: Arc<Client>, ip: IpAddr) -> bool {
    let ident = PingIdentifier(NEXT_IDENT.fetch_add(1, Ordering::Relaxed));
    let mut pinger = client.pinger(ip, ident).await;
    pinger.timeout(METHOD_TIMEOUT);
    pinger.ping(PingSequence(0), &[]).await.is_ok()
}

async fn tcp_ip(ip: IpAddr, port: u16) -> bool {
    let addr = SocketAddr::new(ip, port);
    matches!(
        timeout(METHOD_TIMEOUT, TcpStream::connect(addr)).await,
        Ok(Ok(_))
    )
}

async fn tcp_host_port(host: &str, port: u16) -> bool {
    let dest = format!("{host}:{port}");
    matches!(
        timeout(METHOD_TIMEOUT, TcpStream::connect(dest)).await,
        Ok(Ok(_))
    )
}

/// Send a tiny HTTP request. Any HTTP response means the site is reachable,
/// even 4xx/5xx — the property answered.
async fn http_head(host: &str, port: u16) -> bool {
    let dest = if host.parse::<std::net::Ipv6Addr>().is_ok() {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let Ok(Ok(mut stream)) = timeout(METHOD_TIMEOUT, TcpStream::connect(&dest)).await else {
        return false;
    };
    let req = format!(
        "HEAD / HTTP/1.0\r\nHost: {host}\r\nUser-Agent: edt-down-for-me/0.1\r\nConnection: close\r\n\r\n"
    );
    if timeout(Duration::from_millis(800), stream.write_all(req.as_bytes()))
        .await
        .ok()
        .and_then(|r| r.ok())
        .is_none()
    {
        return false;
    }
    let mut buf = [0u8; 12];
    let n = timeout(Duration::from_millis(800), stream.read(&mut buf))
        .await
        .ok()
        .and_then(|r| r.ok())
        .unwrap_or(0);
    n >= 5 && buf.starts_with(b"HTTP/")
}
