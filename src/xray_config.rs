use anyhow::{Context as _, Result};
use litemap::LiteMap;
use serde_json::{Value, json};

use crate::proxy_config::ProxyConfig;

/// # Errors
/// Will result error if proxy config is invalid
pub fn generate_xray_config(proxies: &[ProxyConfig], base_port: usize) -> Result<String> {
    let mut inbounds = Vec::new();
    let mut outbounds = Vec::new();
    let mut rules = Vec::new();

    for (i, proxy) in proxies.iter().enumerate() {
        let port = base_port + i;
        let inbound_tag = format!("socks-in-{i}");

        inbounds.push(json!({
            "listen": "127.0.0.1",
            "port": port,
            "protocol": "socks",
            "settings": {"auth": "noauth", "udp": true},
            "tag": inbound_tag.clone()
        }));

        if let Some(outbound) = create_outbound(proxy, i)? {
            outbounds.push(outbound);
            rules.push(json!({
                "type": "field",
                "inboundTag": [inbound_tag],
                "outboundTag": format!("{}-out-{i}", proxy.protocol)
            }));
        }
    }

    outbounds.push(json!({
        "protocol": "freedom",
        "tag": "direct"
    }));

    let config = json!({
        "log": {"loglevel": "error"},
        "inbounds": inbounds,
        "outbounds": outbounds,
        "routing": {
            "domainStrategy": "IPIfNonMatch",
            "rules": rules
        }
    });

    serde_json::to_string_pretty(&config).context("Failed to serialize Xray config")
}

fn create_outbound(proxy: &ProxyConfig, index: usize) -> Result<Option<Value>> {
    let outbound = match proxy.protocol.as_str() {
        "http" | "https" => create_http_outbound(proxy, index),
        "socks" | "socks5" => create_socks_outbound(proxy, index),
        "ss" | "shadowsocks" => create_shadowsocks_outbound(proxy, index),
        "trojan" => create_trojan_outbound(proxy, index),
        "vless" => create_vless_outbound(proxy, index),
        "vmess" => create_vmess_outbound(proxy, index),
        _ => return Err(anyhow::anyhow!("Unsupported protocol: {}", proxy.protocol)),
    };

    Ok(Some(outbound))
}

fn create_http_outbound(proxy: &ProxyConfig, index: usize) -> Value {
    let settings = create_common_server_settings(proxy, &["user", "pass"]);
    json!({
        "protocol": "http",
        "settings": settings,
        "tag": format!("http-out-{index}")
    })
}

fn create_socks_outbound(proxy: &ProxyConfig, index: usize) -> Value {
    let settings = create_common_server_settings(proxy, &["user", "pass"]);
    json!({
        "protocol": "socks",
        "settings": settings,
        "tag": format!("socks-out-{index}")
    })
}

fn create_shadowsocks_outbound(proxy: &ProxyConfig, index: usize) -> Value {
    let mut settings = create_common_server_settings(proxy, &[]);

    if let Some(method) = proxy.query_params.get("method") {
        settings["method"] = json!(method);
    } else {
        settings["method"] = json!("aes-256-gcm");
    }

    settings["password"] = json!(proxy.username);

    if let Some(uot) = proxy.query_params.get("uot") {
        settings["uot"] = json!(uot == "true");
    }
    if let Some(uot_version) = proxy
        .query_params
        .get("UoTVersion")
        .and_then(|v| v.parse::<u32>().ok())
    {
        settings["UoTVersion"] = json!(uot_version);
    }

    json!({
        "protocol": "shadowsocks",
        "settings": settings,
        "tag": format!("ss-out-{index}")
    })
}

fn create_trojan_outbound(proxy: &ProxyConfig, index: usize) -> Value {
    let mut settings = create_common_server_settings(proxy, &[]);
    settings["password"] = json!(proxy.username);

    let mut outbound = json!({
        "protocol": "trojan",
        "settings": settings,
        "tag": format!("trojan-out-{index}")
    });

    if let Some(stream_settings) = create_stream_settings(&proxy.query_params) {
        outbound["streamSettings"] = stream_settings;
    }

    outbound
}

fn create_vless_outbound(proxy: &ProxyConfig, index: usize) -> Value {
    let mut settings = create_common_server_settings(proxy, &[]);
    settings["id"] = json!(proxy.username);
    settings["encryption"] = json!("none");

    if let Some(flow) = proxy.query_params.get("flow") {
        settings["flow"] = json!(flow);
    }

    let mut outbound = json!({
        "protocol": "vless",
        "settings": settings,
        "tag": format!("vless-out-{index}")
    });

    if let Some(stream_settings) = create_stream_settings(&proxy.query_params) {
        outbound["streamSettings"] = stream_settings;
    }

    outbound
}

fn create_vmess_outbound(proxy: &ProxyConfig, index: usize) -> Value {
    let security = proxy
        .query_params
        .get("security")
        .map(|s| s.as_str())
        .unwrap_or("auto");
    let level = proxy
        .query_params
        .get("level")
        .and_then(|l| l.parse().ok())
        .unwrap_or(0);

    let settings = json!({
        "vnext": [{
            "address": proxy.address,
            "port": proxy.port,
            "users": [{
                "id": proxy.username,
                "security": security,
                "level": level
            }]
        }]
    });

    let mut outbound = json!({
        "protocol": "vmess",
        "settings": settings,
        "tag": format!("vmess-out-{index}")
    });

    if let Some(stream_settings) = create_stream_settings(&proxy.query_params) {
        outbound["streamSettings"] = stream_settings;
    }

    outbound
}

fn create_common_server_settings(proxy: &ProxyConfig, additional_fields: &[&str]) -> Value {
    let mut settings = json!({
        "address": proxy.address,
        "port": proxy.port
    });

    let all_fields = [
        ("user", "user"),
        ("pass", "pass"),
        ("level", "level"),
        ("email", "email"),
    ];

    for (param, field) in all_fields
        .iter()
        .copied()
        .chain(additional_fields.iter().map(|&f| (f, f)))
    {
        if let Some(value) = proxy.query_params.get(param) {
            settings[field] = if param == "level" {
                json!(value.parse::<u32>().unwrap_or(0))
            } else {
                json!(value)
            };
        }
    }

    settings
}

fn create_stream_settings(query_params: &LiteMap<String, String>) -> Option<Value> {
    let security = query_params
        .get("security")
        .map(|s| s.as_str())
        .unwrap_or("none");
    let network = query_params
        .get("type")
        .map(|s| s.as_str())
        .unwrap_or("tcp");

    if security == "none" || ["http", "h2"].contains(&network) {
        return None;
    }

    let mut stream_settings = json!({
        "network": network,
        "security": security,
    });

    match security {
        "reality" => {
            if let Some(reality_settings) = create_reality_settings(query_params) {
                stream_settings["realitySettings"] = reality_settings;
            } else {
                return None;
            }
        }
        "tls" => {
            if let Some(tls_settings) = create_tls_settings(query_params) {
                stream_settings["tlsSettings"] = tls_settings;
            } else {
                return None;
            }
        }
        _ => {}
    }

    apply_network_settings(&mut stream_settings, query_params, network);

    Some(stream_settings)
}

fn create_reality_settings(query_params: &LiteMap<String, String>) -> Option<Value> {
    let required = ["sni", "pbk", "sid"];
    if !required
        .iter()
        .all(|&param| query_params.get(param).is_some())
    {
        return None;
    }

    let mut settings = json!({});

    for &param in &required {
        if let Some(value) = query_params.get(param) {
            let clean_value = if param == "sid" {
                normalize_shortid(value)
            } else {
                value.split('=').next().unwrap_or(value).to_owned()
            };
            settings[map_reality_field(param)] = json!(clean_value);
        }
    }

    // Optional fields
    let optional_fields = [
        ("fingerprint", "fp"),
        ("spiderX", "spx"),
        ("privateKey", "privateKey"),
    ];

    for (field, param) in optional_fields {
        if let Some(value) = query_params.get(param) {
            settings[field] = json!(value);
        }
    }

    if let Some(xver) = query_params.get("xver").and_then(|v| v.parse::<u32>().ok()) {
        settings["xver"] = json!(xver);
    }

    Some(settings)
}

fn normalize_shortid(shortid: &str) -> String {
    let s = shortid.trim();
    let s = if s.len() % 2 == 1 {
        format!("0{s}")
    } else {
        s.to_owned()
    };
    let s = s.chars().take(16).collect::<String>();
    if s.chars().all(|c| c.is_ascii_hexdigit()) {
        s
    } else {
        "00".to_owned()
    }
}

fn create_tls_settings(query_params: &LiteMap<String, String>) -> Option<Value> {
    let mut settings = json!({});

    if let Some(sni) = query_params.get("sni") {
        settings["serverName"] = json!(sni);
    } else {
        return None;
    }

    if let Some(alpn) = query_params.get("alpn") {
        let alpn_list: Vec<&str> = alpn.split(',').collect();
        settings["alpn"] = json!(alpn_list);
    }

    if let Some(fp) = query_params.get("fp") {
        settings["fingerprint"] = json!(fp);
    }

    Some(settings)
}

fn apply_network_settings(
    stream_settings: &mut Value,
    query_params: &LiteMap<String, String>,
    network: &str,
) {
    match network {
        "ws" => {
            if let Some(path) = query_params.get("path") {
                let decoded_path = percent_encoding::percent_decode_str(path).decode_utf8_lossy();
                let mut ws_settings = json!({ "path": decoded_path });

                if let Some(host) = query_params.get("host") {
                    ws_settings["headers"] = json!({ "Host": host });
                }

                stream_settings["wsSettings"] = ws_settings;
            }
        }
        "grpc" => {
            if let Some(service_name) = query_params.get("serviceName") {
                stream_settings["grpcSettings"] = json!({ "serviceName": service_name });
            }
        }
        "xhttp" => {
            let mut xhttp_settings = serde_json::Map::new();

            if let Some(path) = query_params.get("path") {
                let decoded_path = percent_encoding::percent_decode_str(path).decode_utf8_lossy();
                xhttp_settings.insert("path".to_owned(), json!(decoded_path));
            }

            if let Some(host) = query_params.get("host") {
                xhttp_settings.insert("host".to_owned(), json!(host));
            }

            let mode = query_params
                .get("mode")
                .map(|s| s.as_str())
                .unwrap_or("auto");
            if mode != "auto" {
                xhttp_settings.insert("mode".to_owned(), json!(mode));
            }

            let extra = create_xhttp_extra(query_params);
            if !extra.is_empty() {
                xhttp_settings.insert("extra".to_owned(), json!(extra));
            }

            if !xhttp_settings.is_empty() {
                stream_settings["xhttpSettings"] = json!(xhttp_settings);
            }
        }
        _ => {}
    }
}

fn create_xhttp_extra(query_params: &LiteMap<String, String>) -> serde_json::Map<String, Value> {
    let mut extra = serde_json::Map::new();

    if let Some(headers) = query_params.get("headers")
        && let Ok(headers_value) = serde_json::from_str::<Value>(headers)
    {
        extra.insert("headers".to_owned(), headers_value);
    }

    let numeric_fields = [
        ("xPaddingBytes", "xPaddingBytes"),
        ("scMaxEachPostBytes", "scMaxEachPostBytes"),
        ("scMinPostsIntervalMs", "scMinPostsIntervalMs"),
        ("scMaxBufferedPosts", "scMaxBufferedPosts"),
    ];

    for (param, field) in numeric_fields {
        if let Some(value) = query_params.get(param).and_then(|v| v.parse::<u32>().ok()) {
            extra.insert(field.to_owned(), json!(value));
        }
    }

    let bool_fields = [
        ("noGRPCHeader", "noGRPCHeader"),
        ("noSSEHeader", "noSSEHeader"),
    ];

    for (param, field) in bool_fields {
        if let Some(value) = query_params.get(param) {
            extra.insert(field.to_owned(), json!(value == "true"));
        }
    }

    if let Some(secs) = query_params.get("scStreamUpServerSecs") {
        extra.insert("scStreamUpServerSecs".to_owned(), json!(secs));
    }

    extra
}

fn map_reality_field(param: &str) -> &str {
    match param {
        "sni" => "serverName",
        "pbk" => "publicKey",
        "sid" => "shortId",
        _ => param,
    }
}
