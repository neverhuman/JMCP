use chrono::Utc;
use jmcp_domain::{AdapterHealth, HealthLevel};
use std::{
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    process::Command,
    time::Duration as StdDuration,
};

pub(crate) fn jeryu_health() -> AdapterHealth {
    if let Ok(url) = std::env::var("JMCP_JERYU_URL") {
        return AdapterHealth {
            name: "jeryu".to_owned(),
            health: health_for_url(&url),
            endpoint: Some(url),
            detail: "configured by JMCP_JERYU_URL".to_owned(),
            checked_at: Utc::now(),
        };
    }

    for url in ["http://127.0.0.1:8799", "http://127.0.0.1:8787"] {
        if health_for_url(url) == HealthLevel::Nominal {
            return AdapterHealth {
                name: "jeryu".to_owned(),
                health: HealthLevel::Nominal,
                endpoint: Some(url.to_owned()),
                detail: "detected local listener".to_owned(),
                checked_at: Utc::now(),
            };
        }
    }

    AdapterHealth {
        name: "jeryu".to_owned(),
        health: HealthLevel::Degraded,
        endpoint: None,
        detail: "Jeryu not detected; JMCP remains available".to_owned(),
        checked_at: Utc::now(),
    }
}

pub(crate) fn jailgun_health() -> AdapterHealth {
    let Some(url) = std::env::var("JMCP_JAILGUN_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return AdapterHealth {
            name: "jailgun".to_owned(),
            health: HealthLevel::Degraded,
            endpoint: None,
            detail: "Jailgun ingest is not configured".to_owned(),
            checked_at: Utc::now(),
        };
    };

    if std::env::var("JMCP_JAILGUN_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return AdapterHealth {
            name: "jailgun".to_owned(),
            health: HealthLevel::Blocked,
            endpoint: Some(url),
            detail: "Jailgun ingest token is not configured".to_owned(),
            checked_at: Utc::now(),
        };
    }

    if !url_has_policy_entry(&url, "JMCP_JAILGUN_ALLOWED_URLS") {
        return AdapterHealth {
            name: "jailgun".to_owned(),
            health: HealthLevel::Blocked,
            endpoint: Some(url),
            detail: "Jailgun ingest endpoint is outside configured local submission policy"
                .to_owned(),
            checked_at: Utc::now(),
        };
    }

    let health = health_for_url(&url);
    AdapterHealth {
        name: "jailgun".to_owned(),
        health,
        endpoint: Some(url),
        detail: match health {
            HealthLevel::Nominal => "Jailgun ingest is configured".to_owned(),
            _ => "Jailgun configured but listener is not reachable".to_owned(),
        },
        checked_at: Utc::now(),
    }
}

fn health_for_url(url: &str) -> HealthLevel {
    let Some(addr) = socket_addr_from_url(url) else {
        return HealthLevel::Degraded;
    };
    if TcpStream::connect_timeout(&addr, StdDuration::from_millis(150)).is_ok() {
        HealthLevel::Nominal
    } else {
        HealthLevel::Degraded
    }
}

fn socket_addr_from_url(url: &str) -> Option<SocketAddr> {
    let without_scheme = match url.strip_prefix("http://") {
        Some(value) => value,
        None => match url.strip_prefix("https://") {
            Some(value) => value,
            None => return None,
        },
    };
    let host_port = without_scheme.split('/').next()?;
    host_port
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
}

fn url_has_policy_entry(url: &str, env_key: &str) -> bool {
    let normalized = url.trim().trim_end_matches('/');
    std::env::var(env_key)
        .ok()
        .map(|allowed| {
            allowed
                .split(',')
                .map(|entry| entry.trim().trim_end_matches('/'))
                .any(|entry| entry == normalized)
        })
        .unwrap_or(false)
}

pub(crate) fn command_available(command: &str) -> HealthLevel {
    match Command::new("sh")
        .args(["-c", &format!("command -v {command}")])
        .output()
    {
        Ok(output) if output.status.success() => HealthLevel::Nominal,
        _ => HealthLevel::Degraded,
    }
}
