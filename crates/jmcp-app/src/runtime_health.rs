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

pub(crate) fn command_available(command: &str) -> HealthLevel {
    match Command::new("sh")
        .args(["-c", &format!("command -v {command}")])
        .output()
    {
        Ok(output) if output.status.success() => HealthLevel::Nominal,
        _ => HealthLevel::Degraded,
    }
}
