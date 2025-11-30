use anyhow::{Context as _, Result};
use clap::Parser;
use futures::{StreamExt as _, TryFutureExt as _, stream};
use log::Level;
use reqwest::{Client, ClientBuilder};
use std::{
    collections::HashSet, fs, net::IpAddr, process::Stdio, str::FromStr as _, sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    process::Command,
    sync::{Mutex, Semaphore},
};
use url::{Host, Url};

use crate::{
    dns_cache::DnsCache, parse_url::parse_proxy_url, proxy_config::ProxyConfig,
    xray_config::generate_xray_config,
};

pub mod dns_cache;
pub mod parse_url;
pub mod proxy_config;
pub mod xray_config;

#[cfg(debug_assertions)]
const CONFIG_FILE: &str = "xconf.json";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "vless")]
    scheme: String,

    #[arg(short, long, default_value = "security=reality")]
    whitelist_params: String,

    // Clear ads and other useless trash
    // (sadly what in xhttp path often place ad)
    #[arg(short, long, default_value = "note,host,spx,authority,path,fp")]
    remove_params: String,

    #[arg(short, long, default_value = "out.txt")]
    out_file: String,

    #[cfg(not(debug_assertions))]
    #[arg(long, default_value = "sources.txt,sources-vless.txt")]
    sources_files: String,

    #[cfg(debug_assertions)]
    #[arg(long, default_value = "sources-debug.txt")]
    sources_files: String,

    #[arg(long, default_value = "resolved.txt")]
    dns_cache_file: String,

    #[arg(long, default_value_t = 300)]
    ping_timeout_ms: u128,

    #[arg(long, default_value_t = 100)]
    ping_delay: u64,

    #[arg(long, default_value_t = 6)]
    ping_count: usize,

    #[arg(long, default_value_t = 1000)]
    request_timeout_ms: u64,

    #[arg(long, default_value_t = 300)]
    chunk_size: usize,

    #[arg(long, default_value_t = 10808)]
    base_start_port: usize,

    #[arg(long, default_value_t = 200)]
    max_concurrent_pings: usize,

    #[arg(long, default_value_t = 100)]
    max_concurrent_checks: usize,

    #[arg(long, default_value_t = 50)]
    max_concurrent_dns: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(Level::Info).context("Logger initialization failed")?;

    let args = Args::parse();
    let param_filters = parse_param_filters(&args.whitelist_params);
    let request_timeout = Duration::from_millis(args.request_timeout_ms);

    let sources_content = args
        .sources_files
        .split(',')
        .filter_map(|src| {
            fs::read_to_string(src)
                .or_else(|_| fs::read_to_string(format!("sources/{src}")))
                .ok()
        })
        .collect::<Vec<_>>()
        .join("\n");

    let proxies = get_proxies_from_sources(&sources_content).await?;

    log::info!("Loaded ~{} proxies", proxies.lines().count());

    let valid_urls = proxies
        .lines()
        .filter_map(|line| {
            parse_proxy_url(
                line,
                &args.scheme,
                &param_filters,
                &args.remove_params.split(',').collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    log::info!("Selected {} proxies", valid_urls.len());

    let dns_cache = Arc::new(Mutex::new(DnsCache::new(&args.dns_cache_file)));
    dns_cache.lock().await.load_cache()?;

    let resolved_proxies = resolve_proxies(valid_urls, dns_cache, args.max_concurrent_dns).await?;

    log::info!("Resolved {} proxies", resolved_proxies.len());

    let pinged_proxies = if args.ping_count > 0 {
        let pinged_proxies = ping_proxies(
            resolved_proxies,
            args.ping_timeout_ms,
            args.ping_delay,
            args.max_concurrent_pings,
            1,
        )
        .await;

        log::info!(
            "Pinged {} proxies, now getting average ping",
            pinged_proxies.len()
        );

        let pinged_proxies = ping_proxies(
            pinged_proxies,
            args.ping_timeout_ms,
            args.ping_delay,
            args.max_concurrent_pings,
            args.ping_count - 1,
        )
        .await;

        log::info!("Pinged {} proxies with average ping", pinged_proxies.len());

        pinged_proxies
    } else {
        resolved_proxies.into_iter().collect::<Vec<_>>()
    };

    let working_proxies = test_proxies_in_chunks(
        &pinged_proxies,
        args.chunk_size,
        args.base_start_port,
        request_timeout,
        args.max_concurrent_checks,
    )
    .await?;

    log::info!("Found {} working proxies", working_proxies.len());

    save_results(&working_proxies, &args.out_file).context("Failed to save results")?;

    Ok(())
}

fn parse_param_filters(params: &str) -> Vec<(&str, &str)> {
    params
        .split(',')
        .filter_map(|param| param.split_once('='))
        .collect()
}

async fn resolve_proxies(
    urls: Vec<Url>,
    dns_cache: Arc<Mutex<DnsCache>>,
    max_concurrent_dns: usize,
) -> Result<HashSet<ProxyConfig>> {
    let semaphore = Arc::new(Semaphore::new(max_concurrent_dns));

    let resolved = stream::iter(urls)
        .map(|url| {
            let dns_cache = Arc::clone(&dns_cache);
            let permit = Arc::clone(&semaphore);
            async move {
                let _permit = permit.acquire().await;
                resolve_and_create_config(url, dns_cache).await
            }
        })
        .buffer_unordered(max_concurrent_dns)
        .filter_map(|result| async { result.ok().flatten() })
        .collect::<Vec<_>>()
        .await;

    dns_cache.lock().await.save()?;

    Ok(HashSet::from_iter(resolved))
}

async fn resolve_and_create_config(
    url: Url,
    dns_cache: Arc<Mutex<DnsCache>>,
) -> Result<Option<ProxyConfig>> {
    let host = url.host().context("URL has no host")?;
    let resolved_addr = resolve_host(host, url.port(), dns_cache).await?;
    Ok(Some(ProxyConfig::from_url(url, resolved_addr)))
}

async fn resolve_host(
    host: Host<&str>,
    port: Option<u16>,
    dns_cache: Arc<Mutex<DnsCache>>,
) -> Result<IpAddr> {
    match host {
        Host::Domain(domain) => {
            let domain_lower = domain.to_lowercase();

            if let Ok(addr) = IpAddr::from_str(&domain_lower) {
                return Ok(addr);
            }

            let cached_addr = dns_cache.lock().await.get(&domain_lower);
            if let Some(addr) = cached_addr {
                return Ok(addr);
            }

            let resolved_addr = tokio::net::lookup_host((
                domain_lower.as_str(),
                port.context("Port required for DNS lookup")?,
            ))
            .await
            .context("DNS lookup failed")?
            .next()
            .context("No addresses found")?
            .ip();

            dns_cache.lock().await.insert(domain_lower, resolved_addr);
            Ok(resolved_addr)
        }
        Host::Ipv4(ip) => Ok(IpAddr::V4(ip)),
        Host::Ipv6(ip) => Ok(IpAddr::V6(ip)),
    }
}

async fn ping_proxies(
    proxies: impl IntoIterator<Item = ProxyConfig>,
    ping_timeout_ms: u128,
    ping_delay: u64,
    max_concurrent_pings: usize,
    ping_count: usize,
) -> Vec<ProxyConfig> {
    stream::iter(proxies)
        .map(|mut proxy| async move {
            let mut total_duration = Duration::from_millis(0);
            let mut successful_count = 0;

            for _ in 0..ping_count {
                if let Ok((_, ping)) = surge_ping::ping(proxy.address, &[]).await
                    && ping.as_millis() < ping_timeout_ms
                {
                    total_duration += ping;
                    successful_count += 1;
                }

                if ping_count > 1 {
                    tokio::time::sleep(Duration::from_millis(ping_delay)).await;
                }
            }

            if successful_count > 0 {
                proxy.ping = total_duration / successful_count as u32;
                Some(proxy)
            } else {
                None
            }
        })
        .buffer_unordered(max_concurrent_pings)
        .filter_map(|x| async { x })
        .collect()
        .await
}

async fn test_proxies_in_chunks(
    pinged_proxies: &[ProxyConfig],
    chunk_size: usize,
    base_start_port: usize,
    request_timeout: Duration,
    max_concurrent_checks: usize,
) -> Result<Vec<ProxyConfig>> {
    let mut all_working = Vec::new();

    for (chunk_index, chunk) in pinged_proxies.chunks(chunk_size).enumerate() {
        let base_port = base_start_port + chunk_index * chunk_size;
        let config = generate_xray_config(chunk, base_port)?;

        let mut xray_process = start_xray_with_config(&config).await?;
        tokio::time::sleep(Duration::from_secs(1)).await;

        if let Some(exit) = xray_process.try_wait()? {
            log::warn!("Xray exited: {exit}");
            if let Some(stdout) = &mut xray_process.stdout {
                let mut out = String::new();
                stdout.read_to_string(&mut out).await?;
                log::warn!("Stdout: {out}");
            }
        }

        let working_chunk =
            test_proxy_chunk(chunk, base_port, request_timeout, max_concurrent_checks).await;
        all_working.extend(working_chunk);

        xray_process.kill().await.ok();
    }

    all_working.sort_by(|a, b| a.ping.cmp(&b.ping));
    Ok(all_working)
}

async fn start_xray_with_config(config: &str) -> Result<tokio::process::Child> {
    #[cfg(debug_assertions)]
    fs::write(CONFIG_FILE, config).context("Failed to write Xray config")?;

    let mut command = Command::new("xray")
        .args(["run", "-config", "stdin:"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to start Xray")?;

    if let Some(mut stdin) = command.stdin.take() {
        stdin
            .write_all(config.as_bytes())
            .await
            .context("Failed to write config to Xray")?;
        stdin.flush().await?;
    }

    Ok(command)
}

async fn test_proxy_chunk(
    chunk: &[ProxyConfig],
    base_port: usize,
    request_timeout: Duration,
    max_concurrent_checks: usize,
) -> Vec<ProxyConfig> {
    let semaphore = Arc::new(Semaphore::new(max_concurrent_checks));

    stream::iter(chunk.iter().enumerate())
        .map(|(i, proxy)| {
            let permit = Arc::clone(&semaphore);
            async move {
                let _permit = permit.acquire().await;
                let port = base_port + i;

                let proxy_client =
                    reqwest::Proxy::all(format!("socks5://127.0.0.1:{port}")).ok()?;
                let client = Client::builder()
                    .timeout(request_timeout)
                    .proxy(proxy_client)
                    .build()
                    .ok()?;

                client
                    .get("https://discord.com")
                    .send()
                    .await
                    .ok()
                    .filter(|response| response.status().is_success())
                    .map(|_| proxy.clone())
            }
        })
        .buffer_unordered(max_concurrent_checks)
        .filter_map(|x| async { x })
        .collect()
        .await
}

async fn get_proxies_from_sources(sources: &str) -> Result<String> {
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(10))
        .build()?;
    let fetch_tasks: Vec<_> = sources
        .lines()
        .filter(|line| line.starts_with("https://"))
        .map(|url| {
            let value = client.clone();
            async move {
                let data = value
                    .get(url)
                    .send()
                    .and_then(|r| async { r.text().await })
                    .await;
                log::info!("Loaded source: {url}");
                data
            }
        })
        .collect();

    let responses = futures::future::join_all(fetch_tasks)
        .await
        .into_iter()
        .filter_map(|x| x.ok())
        .collect::<Vec<_>>();

    Ok(responses.join("\n"))
}

fn save_results(working_proxies: &[ProxyConfig], results_file: &str) -> Result<()> {
    let mut sorted_proxies = working_proxies.to_vec();
    sorted_proxies.sort_by(|a, b| a.ping.cmp(&b.ping));

    let results = sorted_proxies
        .iter()
        .enumerate()
        .map(|(id, proxy)| {
            log::info!("{}ms - {proxy}", proxy.ping.as_millis());
            format!(
                "{proxy}#Novaprox - {} [{}ms]",
                id + 1,
                proxy.ping.as_millis()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(results_file, results).context("Failed to write results")
}
