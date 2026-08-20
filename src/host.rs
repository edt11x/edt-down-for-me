//! Parse and validate hostnames and IP addresses.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Canonical form stored in config and shown in the UI.
    pub display: String,
    /// Hostname or IP without brackets.
    pub host: String,
    /// Optional non-default port.
    pub port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    Invalid,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Empty => write!(f, "Enter a host or IP"),
            ParseError::Invalid => write!(f, "Invalid host or IP"),
        }
    }
}

/// Parse user input into a canonical target.
///
/// Accepts hostnames, IPv4, IPv6, optional `http(s)://` prefix, path, and port.
pub fn parse_target(raw: &str) -> Result<Target, ParseError> {
    let mut s = raw.trim();
    if s.is_empty() {
        return Err(ParseError::Empty);
    }
    if s.contains(char::is_whitespace) {
        return Err(ParseError::Invalid);
    }

    let lower = s.to_ascii_lowercase();
    if let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .or_else(|| lower.strip_prefix("//"))
    {
        s = &s[s.len() - rest.len()..];
    }

    if let Some(at) = s.rfind('@') {
        s = &s[at + 1..];
    }

    if let Some(slash) = s.find('/') {
        s = &s[..slash];
    }
    if s.is_empty() {
        return Err(ParseError::Invalid);
    }

    if s.starts_with('[') {
        return parse_bracketed_ipv6(s);
    }

    if let Ok(v6) = Ipv6Addr::from_str(s) {
        return Ok(ip_target(IpAddr::V6(v6), None));
    }

    if let Some((host_part, port_part)) = split_host_port(s) {
        let port = parse_port(port_part)?;
        return finish_host(host_part, Some(port));
    }

    finish_host(s, None)
}

fn parse_bracketed_ipv6(s: &str) -> Result<Target, ParseError> {
    let end = s.find(']').ok_or(ParseError::Invalid)?;
    let inner = &s[1..end];
    let rest = &s[end + 1..];
    let ip = Ipv6Addr::from_str(inner).map_err(|_| ParseError::Invalid)?;
    let port = if rest.is_empty() {
        None
    } else if let Some(p) = rest.strip_prefix(':') {
        Some(parse_port(p)?)
    } else {
        return Err(ParseError::Invalid);
    };
    Ok(ip_target(IpAddr::V6(ip), port))
}

fn split_host_port(s: &str) -> Option<(&str, &str)> {
    let (left, right) = s.rsplit_once(':')?;
    if left.is_empty() || right.is_empty() {
        return None;
    }
    if !right.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if Ipv6Addr::from_str(s).is_ok() {
        return None;
    }
    Some((left, right))
}

fn parse_port(s: &str) -> Result<u16, ParseError> {
    let port: u16 = s.parse().map_err(|_| ParseError::Invalid)?;
    if port == 0 {
        return Err(ParseError::Invalid);
    }
    Ok(port)
}

fn finish_host(host: &str, port: Option<u16>) -> Result<Target, ParseError> {
    if let Ok(v4) = Ipv4Addr::from_str(host) {
        return Ok(ip_target(IpAddr::V4(v4), port));
    }
    if let Ok(v6) = Ipv6Addr::from_str(host) {
        return Ok(ip_target(IpAddr::V6(v6), port));
    }
    if !is_valid_hostname(host) {
        return Err(ParseError::Invalid);
    }
    let host = host.to_ascii_lowercase();
    Ok(Target {
        display: format_display(&host, false, port),
        host,
        port: normalize_port(port),
    })
}

fn ip_target(ip: IpAddr, port: Option<u16>) -> Target {
    let host = ip.to_string();
    let is_v6 = matches!(ip, IpAddr::V6(_));
    Target {
        display: format_display(&host, is_v6, port),
        host,
        port: normalize_port(port),
    }
}

fn normalize_port(port: Option<u16>) -> Option<u16> {
    match port {
        Some(80) | Some(443) | None => None,
        other => other,
    }
}

fn format_display(host: &str, is_v6: bool, port: Option<u16>) -> String {
    let port = normalize_port(port);
    match (is_v6, port) {
        (true, Some(p)) => format!("[{host}]:{p}"),
        (true, None) => host.to_string(),
        (false, Some(p)) => format!("{host}:{p}"),
        (false, None) => host.to_string(),
    }
}

fn is_valid_hostname(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 || host.starts_with('.') || host.ends_with('.') {
        return false;
    }
    host.split('.').all(is_valid_label)
}

fn is_valid_label(label: &str) -> bool {
    if label.is_empty() || label.len() > 63 {
        return false;
    }
    let bytes = label.as_bytes();
    if !bytes[0].is_ascii_alphanumeric() || !bytes[bytes.len() - 1].is_ascii_alphanumeric() {
        return false;
    }
    bytes.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'-')
}

impl Target {
    pub fn tcp_host(&self) -> String {
        if self.host.parse::<Ipv6Addr>().is_ok() {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        }
    }

    pub fn probe_ports(&self) -> Vec<u16> {
        match self.port {
            Some(p) => vec![p],
            None => vec![443, 80],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_hostname() {
        let t = parse_target("Google.COM").unwrap();
        assert_eq!(t.display, "google.com");
        assert_eq!(t.host, "google.com");
        assert_eq!(t.port, None);
    }

    #[test]
    fn parses_url() {
        let t = parse_target("https://github.com/foo").unwrap();
        assert_eq!(t.display, "github.com");
        let t = parse_target("HTTPS://GitHub.COM/foo").unwrap();
        assert_eq!(t.display, "github.com");
    }

    #[test]
    fn parses_ipv4() {
        let t = parse_target("8.8.8.8").unwrap();
        assert_eq!(t.display, "8.8.8.8");
    }

    #[test]
    fn parses_ipv4_with_port() {
        let t = parse_target("8.8.8.8:53").unwrap();
        assert_eq!(t.display, "8.8.8.8:53");
        assert_eq!(t.port, Some(53));
    }

    #[test]
    fn default_ports_are_stripped() {
        let t = parse_target("example.com:443").unwrap();
        assert_eq!(t.display, "example.com");
        assert_eq!(t.port, None);
    }

    #[test]
    fn parses_ipv6() {
        let t = parse_target("2001:4860:4860::8888").unwrap();
        assert_eq!(t.host, "2001:4860:4860::8888");
        assert_eq!(t.display, "2001:4860:4860::8888");
    }

    #[test]
    fn parses_bracketed_ipv6_with_port() {
        let t = parse_target("[::1]:8080").unwrap();
        assert_eq!(t.display, "[::1]:8080");
        assert_eq!(t.port, Some(8080));
    }

    #[test]
    fn rejects_empty_and_garbage() {
        assert_eq!(parse_target("  "), Err(ParseError::Empty));
        assert!(parse_target("not a host").is_err());
        assert!(parse_target("http://").is_err());
        assert!(parse_target("-bad.com").is_err());
    }

    #[test]
    fn accepts_localhost() {
        assert_eq!(parse_target("localhost").unwrap().display, "localhost");
    }
}
