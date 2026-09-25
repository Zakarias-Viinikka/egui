use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug)]
pub enum BanError {
    Secrets(String),
    Refresh(String),
    ResolveSelf(String),
    Ban(String),
}

impl std::fmt::Display for BanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BanError::Secrets(s) => write!(f, "secrets: {s}"),
            BanError::Refresh(s) => write!(f, "token refresh: {s}"),
            BanError::ResolveSelf(s) => write!(f, "resolve self: {s}"),
            BanError::Ban(s) => write!(f, "ban: {s}"),
        }
    }
}

impl std::error::Error for BanError {}

struct Secrets {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    path: PathBuf,
}

fn ureq_err_string(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_else(|_| "<no body>".to_string());
            format!("HTTP {code}: {body}")
        }
        ureq::Error::Transport(t) => format!("transport: {t}"),
    }
}

fn read_secrets() -> Result<Secrets, BanError> {
    let home = std::env::var("HOME")
        .map_err(|e| BanError::Secrets(format!("HOME not set: {e}")))?;
    let path = PathBuf::from(home).join(".config/twitch_chat/secrets.env");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| BanError::Secrets(format!("read {} failed: {e}", path.display())))?;
    let mut client_id: Option<String> = None;
    let mut client_secret: Option<String> = None;
    let mut refresh_token: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim().to_string();
            match k {
                "TWITCH_CLIENT_ID" => client_id = Some(v),
                "TWITCH_CLIENT_SECRET" => client_secret = Some(v),
                "TWITCH_REFRESH_TOKEN" => refresh_token = Some(v),
                _ => {}
            }
        }
    }
    let client_id = client_id
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BanError::Secrets("TWITCH_CLIENT_ID missing or empty".into()))?;
    let client_secret = client_secret
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BanError::Secrets("TWITCH_CLIENT_SECRET missing or empty".into()))?;
    let refresh_token = refresh_token
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BanError::Secrets("TWITCH_REFRESH_TOKEN missing or empty".into()))?;
    Ok(Secrets {
        client_id,
        client_secret,
        refresh_token,
        path,
    })
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
}

fn refresh_access_token(secrets: &mut Secrets) -> Result<String, BanError> {
    let params = [
        ("client_id", secrets.client_id.as_str()),
        ("client_secret", secrets.client_secret.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", secrets.refresh_token.as_str()),
    ];
    let resp: TokenResponse = ureq::post("https://id.twitch.tv/oauth2/token")
        .send_form(&params)
        .map_err(|e| BanError::Refresh(ureq_err_string(e)))?
        .into_json()
        .map_err(|e| BanError::Refresh(format!("response parse failed: {e}")))?;

    if let Some(new_token) = resp.refresh_token {
        if new_token != secrets.refresh_token {
            rewrite_refresh_token(&secrets.path, &new_token)?;
            secrets.refresh_token = new_token;
        }
    }
    Ok(resp.access_token)
}

fn rewrite_refresh_token(path: &Path, new_token: &str) -> Result<(), BanError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| BanError::Refresh(format!("reread {} failed: {e}", path.display())))?;
    let mut out = String::with_capacity(text.len() + new_token.len());
    let mut replaced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("TWITCH_REFRESH_TOKEN=") {
            out.push_str("TWITCH_REFRESH_TOKEN=");
            out.push_str(new_token);
            out.push('\n');
            replaced = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        return Err(BanError::Refresh(
            "TWITCH_REFRESH_TOKEN line not found to rewrite".into(),
        ));
    }
    std::fs::write(path, out)
        .map_err(|e| BanError::Refresh(format!("write {} failed: {e}", path.display())))?;
    Ok(())
}

#[derive(Deserialize)]
struct UsersResponse {
    data: Vec<UserEntry>,
}

#[derive(Deserialize)]
struct UserEntry {
    id: String,
}

fn resolve_self_user_id(access_token: &str, client_id: &str) -> Result<String, BanError> {
    let resp: UsersResponse = ureq::get("https://api.twitch.tv/helix/users")
        .set("Authorization", &format!("Bearer {access_token}"))
        .set("Client-Id", client_id)
        .call()
        .map_err(|e| BanError::ResolveSelf(ureq_err_string(e)))?
        .into_json()
        .map_err(|e| BanError::ResolveSelf(format!("response parse failed: {e}")))?;
    resp.data
        .into_iter()
        .next()
        .map(|u| u.id)
        .ok_or_else(|| BanError::ResolveSelf("response had no data".into()))
}

fn post_ban(
    access_token: &str,
    client_id: &str,
    self_id: &str,
    target_user_id: &str,
) -> Result<(), BanError> {
    let url = format!(
        "https://api.twitch.tv/helix/moderation/bans?broadcaster_id={id}&moderator_id={id}",
        id = self_id
    );
    let body = serde_json::json!({
        "data": { "user_id": target_user_id }
    });
    match ureq::post(&url)
        .set("Authorization", &format!("Bearer {access_token}"))
        .set("Client-Id", client_id)
        .send_json(body)
    {
        Ok(_) => Ok(()),
        Err(e) => Err(BanError::Ban(ureq_err_string(e))),
    }
}

pub fn ban_user_permanently(target_user_id: &str) -> Result<(), BanError> {
    let mut secrets = read_secrets()?;
    let access_token = refresh_access_token(&mut secrets)?;
    let self_id = resolve_self_user_id(&access_token, &secrets.client_id)?;
    post_ban(&access_token, &secrets.client_id, &self_id, target_user_id)?;
    Ok(())
}
