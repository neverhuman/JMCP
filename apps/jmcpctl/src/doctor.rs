use anyhow::Result;
use std::process::Command as StdCommand;

const DEFAULT_API_BIND: &str = "127.0.0.1:18877";
const DEFAULT_COCKPIT_HOST: &str = "127.0.0.1";
const DEFAULT_COCKPIT_PORT: u16 = 15873;
const JERYU_PROTECTED_PORTS: &[u16] = &[2224, 8787, 8799, 8929, 18787, 18788, 19800];

pub(crate) fn doctor_env(server: &str) -> Result<()> {
    let api_bind = match std::env::var("JMCP_API_BIND") {
        Ok(value) => value,
        Err(_) => DEFAULT_API_BIND.to_owned(),
    };
    let cockpit_host = match std::env::var("JMCP_COCKPIT_HOST") {
        Ok(value) => value,
        Err(_) => DEFAULT_COCKPIT_HOST.to_owned(),
    };
    let mut failed = false;
    let cockpit_port_value = match std::env::var("JMCP_COCKPIT_PORT") {
        Ok(value) => value,
        Err(_) => DEFAULT_COCKPIT_PORT.to_string(),
    };
    let cockpit_port = match parse_port(&cockpit_port_value) {
        Ok(port) => port,
        Err(message) => {
            eprintln!("error: JMCP_COCKPIT_PORT {message}");
            failed = true;
            0
        }
    };

    println!("JMCP_API_BIND={api_bind}");
    println!("JMCP_API_URL={server}");
    println!("JMCP_COCKPIT_HOST={cockpit_host}");
    println!("JMCP_COCKPIT_PORT={cockpit_port}");

    if let Some(port) = port_from_bind(&api_bind) {
        if JERYU_PROTECTED_PORTS.contains(&port) {
            eprintln!("error: JMCP_API_BIND uses Jeryu protected port {port}");
            failed = true;
        }
        if let Some(owner) = listener_owner(port) {
            println!("api bind conflict on port {port}: {owner}");
        }
    } else {
        eprintln!("error: could not parse JMCP_API_BIND={api_bind}");
        failed = true;
    }

    if cockpit_port != 0 && JERYU_PROTECTED_PORTS.contains(&cockpit_port) {
        eprintln!("error: JMCP_COCKPIT_PORT uses Jeryu protected port {cockpit_port}");
        failed = true;
    }
    if cockpit_port != 0 {
        if let Some(owner) = listener_owner(cockpit_port) {
            println!("cockpit bind conflict on port {cockpit_port}: {owner}");
        }
    }

    for port in JERYU_PROTECTED_PORTS {
        if let Some(owner) = listener_owner(*port) {
            println!("Jeryu protected port {port} is occupied by: {owner}");
        }
    }

    if listener_owner(8799).is_none() && listener_owner(8787).is_none() {
        eprintln!("warning: Jeryu was not detected on 127.0.0.1:8799 or 127.0.0.1:8787");
    }

    if failed {
        anyhow::bail!("JMCP environment is not safe");
    }
    println!("JMCP environment is safe for Jeryu coexistence");
    Ok(())
}

fn parse_port(value: &str) -> Result<u16, String> {
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(format!("is not numeric: {value}"));
    }
    value
        .parse::<u16>()
        .map_err(|_| format!("is outside the valid TCP port range: {value}"))
}

fn port_from_bind(bind: &str) -> Option<u16> {
    parse_port(bind.rsplit_once(':')?.1).ok()
}

fn listener_owner(port: u16) -> Option<String> {
    let output = StdCommand::new("ss").args(["-ltnp"]).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find(|line| line.contains(&format!(":{port} ")))
        .map(|line| line.trim().to_owned())
}
