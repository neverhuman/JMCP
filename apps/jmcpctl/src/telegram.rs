use anyhow::Result;
use jmcp_approval_telegram::{TelegramBotClient, TelegramConfig};
use std::collections::BTreeSet;
use std::path::PathBuf;

pub(crate) async fn telegram_doctor(env_file: PathBuf, offset_file: PathBuf) -> Result<()> {
    let config = TelegramConfig::from_env_file_for_setup(&env_file)?;
    let mut failed = false;

    println!("JMCP_TELEGRAM_ENV={}", env_file.display());
    println!("telegram_token=loaded (redacted)");
    println!("telegram_api_base={}", config.api_base);
    println!(
        "telegram_allowlist=user_ids:{} chat_ids:{}",
        config.allowed_user_ids.len(),
        config.allowed_chat_ids.len()
    );
    println!("telegram_config={config:?}");

    if !config.has_allowlist() {
        eprintln!("error: telegram allowlist missing");
        failed = true;
    }

    let client = TelegramBotClient::new(config);
    match client.get_me().await {
        Ok(me) => {
            println!(
                "telegram_getMe=ok id:{} username:{}",
                me.id,
                username_or_none(me.username)
            );
        }
        Err(err) => {
            eprintln!("error: telegram getMe failed: {err}");
            failed = true;
        }
    }

    match std::fs::read_to_string(&offset_file) {
        Ok(contents) => match contents.trim().parse::<i64>() {
            Ok(offset) => println!(
                "telegram_offset_file={} offset={offset}",
                offset_file.display()
            ),
            Err(_) => {
                eprintln!(
                    "error: telegram offset file is not a valid integer: {}",
                    offset_file.display()
                );
                failed = true;
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!(
                "telegram_offset_file={} status=absent",
                offset_file.display()
            );
        }
        Err(err) => {
            eprintln!(
                "error: telegram offset file could not be read: {}: {err}",
                offset_file.display()
            );
            failed = true;
        }
    }

    if failed {
        anyhow::bail!("Telegram setup is not ready");
    }
    println!("Telegram setup is ready");
    Ok(())
}

pub(crate) async fn telegram_discover_ids(env_file: PathBuf) -> Result<()> {
    let config = TelegramConfig::from_env_file_for_setup(&env_file)?;
    let client = TelegramBotClient::new(config);
    let updates = client.get_updates(None, 0).await?;
    let mut candidates = BTreeSet::new();
    for update in updates {
        if let Some(message) = update.message {
            if let Some(user) = message.from {
                candidates.insert(format!(
                    "user_id={} chat_id={} chat_type={} username={} first_name={}",
                    user.id,
                    message.chat.id,
                    message.chat.kind,
                    username_or_none(user.username),
                    user.first_name
                ));
            }
        }
    }

    if candidates.is_empty() {
        println!("No Telegram updates found. Send the bot a message, then rerun discover-ids.");
    } else {
        for candidate in candidates {
            println!("{candidate}");
        }
    }
    Ok(())
}

fn username_or_none(username: Option<String>) -> String {
    match username {
        Some(username) => username,
        None => "(none)".to_owned(),
    }
}
