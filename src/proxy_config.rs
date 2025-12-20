use std::{
    fmt::{self},
    net::IpAddr,
    time::Duration,
};

use litemap::LiteMap;
use url::Url;

const DEFAULT_PORTS: &[(&str, u16)] = &[
    ("http", 80),
    ("https", 80),
    ("socks", 1080),
    ("socks5", 1080),
    ("shadowsocks", 8388),
    ("trojan", 443),
    ("vless", 443),
    ("vmess", 443),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    pub address: IpAddr,
    pub port: u16,
    pub protocol: String,
    pub query_params: LiteMap<String, String>,
    pub username: String,
    pub ping: Duration,
}

impl fmt::Display for ProxyConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            address,
            port,
            protocol,
            query_params,
            username,
            ping: _,
        } = self;

        write!(f, "{protocol}://{username}@{address}:{port}")?;

        if !query_params.is_empty() {
            write!(f, "?")?;

            let mut first = true;
            for (key, value) in self.query_params.iter() {
                if !first {
                    write!(f, "&")?;
                }
                write!(f, "{key}={value}")?;
                first = false;
            }
        }

        Ok(())
    }
}

impl std::hash::Hash for ProxyConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.address.hash(state);
        self.port.hash(state);
        self.protocol.hash(state);
        self.query_params.get("sni").hash(state);
        self.query_params.get("pbk").hash(state);
        self.query_params.get("extra").hash(state);
        self.username.hash(state);
    }
}

impl ProxyConfig {
    #[must_use]
    pub fn from_url(url: Url, resolved_addr: IpAddr) -> Self {
        let query_params = url.query_pairs().into_owned().collect::<LiteMap<_, _>>();

        let default_port = DEFAULT_PORTS
            .iter()
            .find(|(scheme, _)| *scheme == url.scheme())
            .map(|(_, port)| *port)
            .unwrap_or(8080);

        Self {
            address: resolved_addr,
            port: url.port().unwrap_or(default_port),
            protocol: url.scheme().to_lowercase(),
            query_params,
            username: url.username().to_lowercase(),
            ping: Duration::default(),
        }
    }
}
