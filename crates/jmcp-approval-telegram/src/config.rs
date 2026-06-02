use crate::TelegramApprovalError;
use std::{collections::HashSet, path::Path};

#[derive(Clone)]
pub struct TelegramConfig {
    token: String,
    pub api_base: String,
    pub allowed_user_ids: HashSet<i64>,
    pub allowed_chat_ids: HashSet<i64>,
}

impl std::fmt::Debug for TelegramConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramConfig")
            .field("token", &"<redacted>")
            .field("api_base", &self.api_base)
            .field("allowed_user_ids", &self.allowed_user_ids)
            .field("allowed_chat_ids", &self.allowed_chat_ids)
            .finish()
    }
}

impl TelegramConfig {
    pub fn from_env_file(path: impl AsRef<Path>) -> Result<Self, TelegramApprovalError> {
        Self::from_env_file_with_allowlist(path, true)
    }

    pub fn from_env_file_for_setup(path: impl AsRef<Path>) -> Result<Self, TelegramApprovalError> {
        Self::from_env_file_with_allowlist(path, false)
    }

    fn from_env_file_with_allowlist(
        path: impl AsRef<Path>,
        require_allowlist: bool,
    ) -> Result<Self, TelegramApprovalError> {
        let mut contents =
            std::fs::read_to_string(path).map_err(|_| TelegramApprovalError::TokenLoadFailed)?;
        append_env_override(&mut contents, "JMCP_TELEGRAM_BOT_TOKEN");
        append_env_override(&mut contents, "TELEGRAM_BOT_TOKEN");
        append_env_override(&mut contents, "BOT_TOKEN");
        append_env_override(&mut contents, "JMCP_TELEGRAM_API_BASE");
        append_env_override(&mut contents, "TELEGRAM_API_BASE");
        append_env_override(&mut contents, "JMCP_TELEGRAM_ALLOWED_USER_IDS");
        append_env_override(&mut contents, "TELEGRAM_ALLOWED_USER_IDS");
        append_env_override(&mut contents, "JMCP_TELEGRAM_ALLOWED_CHAT_IDS");
        append_env_override(&mut contents, "TELEGRAM_ALLOWED_CHAT_IDS");
        Self::from_env_contents_with_allowlist(&contents, require_allowlist)
    }

    pub fn from_env_contents(contents: &str) -> Result<Self, TelegramApprovalError> {
        Self::from_env_contents_with_allowlist(contents, true)
    }

    pub fn from_env_contents_for_setup(contents: &str) -> Result<Self, TelegramApprovalError> {
        Self::from_env_contents_with_allowlist(contents, false)
    }

    fn from_env_contents_with_allowlist(
        contents: &str,
        require_allowlist: bool,
    ) -> Result<Self, TelegramApprovalError> {
        let mut token = None;
        let mut api_base = None;
        let mut allowed_user_ids = HashSet::new();
        let mut allowed_chat_ids = HashSet::new();

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let value = value.trim().trim_matches('"').trim_matches('\'');
                match key.trim() {
                    "TELEGRAM_BOT_TOKEN" | "BOT_TOKEN" | "JMCP_TELEGRAM_BOT_TOKEN" => {
                        token = Some(value.to_owned());
                    }
                    "TELEGRAM_API_BASE" | "JMCP_TELEGRAM_API_BASE" => {
                        api_base = Some(value.trim_end_matches('/').to_owned());
                    }
                    "TELEGRAM_ALLOWED_USER_IDS" | "JMCP_TELEGRAM_ALLOWED_USER_IDS" => {
                        allowed_user_ids.extend(parse_id_list(value)?);
                    }
                    "TELEGRAM_ALLOWED_CHAT_IDS" | "JMCP_TELEGRAM_ALLOWED_CHAT_IDS" => {
                        allowed_chat_ids.extend(parse_id_list(value)?);
                    }
                    _ => {}
                }
            } else if token.is_none() {
                token = Some(line.to_owned());
            }
        }

        let token = token
            .filter(|value| !value.is_empty())
            .ok_or(TelegramApprovalError::MissingToken)?;
        if require_allowlist && allowed_user_ids.is_empty() && allowed_chat_ids.is_empty() {
            return Err(TelegramApprovalError::MissingAllowlist);
        }
        Ok(Self {
            token,
            api_base: api_base.unwrap_or_else(|| "https://api.telegram.org".to_owned()),
            allowed_user_ids,
            allowed_chat_ids,
        })
    }

    pub(crate) fn method_url(&self, method: &str) -> String {
        format!("{}/bot{}/{}", self.api_base, self.token, method)
    }

    pub fn is_allowed(&self, user_id: i64, chat_id: i64) -> bool {
        if self.allowed_user_ids.is_empty() && self.allowed_chat_ids.is_empty() {
            return false;
        }
        (self.allowed_user_ids.is_empty() || self.allowed_user_ids.contains(&user_id))
            && (self.allowed_chat_ids.is_empty() || self.allowed_chat_ids.contains(&chat_id))
    }

    pub fn has_allowlist(&self) -> bool {
        !self.allowed_user_ids.is_empty() || !self.allowed_chat_ids.is_empty()
    }
}

fn parse_id_list(value: &str) -> Result<Vec<i64>, TelegramApprovalError> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse()
                .map_err(|_| TelegramApprovalError::InvalidAllowlist)
        })
        .collect()
}

fn append_env_override(contents: &mut String, key: &str) {
    if let Ok(value) = std::env::var(key) {
        contents.push('\n');
        contents.push_str(key);
        contents.push('=');
        contents.push_str(&value);
    }
}
