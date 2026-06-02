pub(crate) const DEFAULT_JEKKO_BASE_URL: &str = "http://127.0.0.1:4317";
pub(crate) const DEFAULT_JNOCCIO_BASE_URL: &str = "http://127.0.0.1:8765";
pub(crate) const DEFAULT_MODEL: &str = "jnoccio/jnoccio-fusion";
pub(crate) const DEFAULT_TIMEOUT_SECS: u64 = 120;

pub(crate) fn env_or(key: &str, default: &str) -> String {
    match std::env::var(key).ok().filter(|value| !value.is_empty()) {
        Some(value) => value,
        None => default.to_owned(),
    }
}
