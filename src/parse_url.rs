use anyhow::{Context as _, Result};
use base64::Engine as _;
use url::Url;

#[must_use]
pub fn parse_proxy_url(
    line: &str,
    target_scheme: &str,
    param_filters: &[(&str, &str)],
    params_remove: &[&str],
) -> Option<Url> {
    let cleaned_line = line.replace("amp;", "");

    if target_scheme == "vmess" && cleaned_line.starts_with("vmess://") {
        parse_vmess_url(&cleaned_line).ok().flatten()
    } else {
        Url::parse(&cleaned_line)
            .ok()
            .filter(|url| {
                url.scheme() == target_scheme
                    && param_filters
                        .iter()
                        .all(|&(pk, pv)| url.query_pairs().any(|(qk, qv)| qk == pk && qv == pv))
            })
            .map(|mut url| {
                url.set_query(Some(
                    &url.query_pairs()
                        .filter(|(k, _)| !params_remove.contains(&k.as_ref()))
                        .filter_map(|(k, v)| {
                            // Fix encryption=none=sometrash\/eeee in urls
                            // (else xray dont starts)
                            if v == "none" || (k == "type" && v == "tcp") {
                                None
                            } else {
                                Some(format!("{k}={}", v.split('=').next()?))
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("&"),
                ));
                url
            })
    }
}

fn parse_vmess_url(url: &str) -> Result<Option<Url>> {
    let base64_part = url.strip_prefix("vmess://").context("Invalid VMESS URL")?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(base64_part)
        .context("Base64 decode failed")?;
    let config_str = String::from_utf8(decoded).context("Invalid UTF-8 in VMESS config")?;

    let config: serde_json::Value =
        serde_json::from_str(&config_str).context("Invalid JSON in VMESS config")?;

    let address = config["add"]
        .as_str()
        .context("Missing address in VMESS config")?;
    let port = config["port"]
        .as_u64()
        .context("Missing port in VMESS config")? as u16;
    let username = config["id"]
        .as_str()
        .context("Missing ID in VMESS config")?;

    let url_str = format!("vmess://{username}@{address}:{port}");
    let url = Url::parse(&url_str).context("Failed to parse VMESS URL")?;

    Ok(Some(url))
}
