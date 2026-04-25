use anyhow::{Result, bail, Context};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Sha256, Digest};
use rand::Rng;
use crate::auth::accounts::{Account, AccountType};

// Microsoft OAuth2 config for Minecraft
const CLIENT_ID: &str = "00000000402b5328"; // Xbox Live / Minecraft client ID (public)
const REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Deserialize)]
pub struct MsTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct XboxLiveResponse {
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "DisplayClaims")]
    pub display_claims: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct XstsResponse {
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "DisplayClaims")]
    pub display_claims: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MinecraftAuthResponse {
    pub access_token: String,
    pub expires_in: Option<i64>,
    pub username: Option<String>,
    pub token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MinecraftProfile {
    pub id: String,
    pub name: String,
    pub skins: Option<Vec<serde_json::Value>>,
}

/// Generate PKCE code verifier and challenge
fn generate_pkce() -> (String, String) {
    let verifier: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(128)
        .map(char::from)
        .collect();

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    (verifier, challenge)
}

/// Returns the URL to open in the browser for Microsoft login
pub fn get_auth_url() -> (String, String) {
    let (verifier, challenge) = generate_pkce();
    let state: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    let url = format!(
        "https://login.live.com/oauth20_authorize.srf\
        ?client_id={CLIENT_ID}\
        &response_type=code\
        &redirect_uri={REDIRECT_URI}\
        &scope={SCOPE}\
        &code_challenge={challenge}\
        &code_challenge_method=S256\
        &state={state}",
        REDIRECT_URI = urlencoding::encode(REDIRECT_URI),
        SCOPE = urlencoding::encode(SCOPE),
    );

    (url, verifier)
}

/// Exchange auth code for Microsoft tokens
pub async fn exchange_code(
    client: &reqwest::Client,
    code: &str,
    verifier: &str,
) -> Result<MsTokenResponse> {
    let params = [
        ("client_id", CLIENT_ID),
        ("code", code),
        ("code_verifier", verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", REDIRECT_URI),
    ];

    let resp = client
        .post("https://login.live.com/oauth20_token.srf")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        bail!("MS token exchange failed: {text}");
    }

    Ok(resp.json().await?)
}

/// Refresh Microsoft access token
pub async fn refresh_ms_token(
    client: &reqwest::Client,
    refresh_token: &str,
) -> Result<MsTokenResponse> {
    let params = [
        ("client_id", CLIENT_ID),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
        ("redirect_uri", REDIRECT_URI),
        ("scope", SCOPE),
    ];

    let resp = client
        .post("https://login.live.com/oauth20_token.srf")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        bail!("MS token refresh failed: {text}");
    }

    Ok(resp.json().await?)
}

/// Authenticate with Xbox Live
async fn authenticate_xbl(
    client: &reqwest::Client,
    ms_access_token: &str,
) -> Result<XboxLiveResponse> {
    let body = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={ms_access_token}")
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let resp = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        bail!("XBL auth failed: {text}");
    }

    Ok(resp.json().await?)
}

/// Get XSTS token
async fn authenticate_xsts(
    client: &reqwest::Client,
    xbl_token: &str,
) -> Result<XstsResponse> {
    let body = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let resp = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await?;
        // XErr 2148916238 = no Xbox account
        if text.contains("2148916238") {
            bail!("This Microsoft account has no Xbox Live profile. Please create one first at xbox.com");
        }
        bail!("XSTS auth failed ({status}): {text}");
    }

    Ok(resp.json().await?)
}

/// Extract UHS (user hash) from Xbox claims
fn extract_uhs(xbl_response: &XboxLiveResponse) -> Result<String> {
    xbl_response
        .display_claims
        .get("xui")
        .and_then(|xui| xui.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("uhs"))
        .and_then(|uhs| uhs.as_str())
        .map(String::from)
        .context("Could not extract UHS from XBL response")
}

/// Authenticate with Minecraft
async fn authenticate_minecraft(
    client: &reqwest::Client,
    uhs: &str,
    xsts_token: &str,
) -> Result<MinecraftAuthResponse> {
    let body = serde_json::json!({
        "identityToken": format!("XBL3.0 x={uhs};{xsts_token}")
    });

    let resp = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        bail!("Minecraft auth failed: {text}");
    }

    Ok(resp.json().await?)
}

/// Fetch Minecraft profile
async fn fetch_minecraft_profile(
    client: &reqwest::Client,
    mc_access_token: &str,
) -> Result<MinecraftProfile> {
    let resp = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(mc_access_token)
        .send()
        .await?;

    if resp.status().as_u16() == 404 {
        bail!("This account does not own Minecraft Java Edition");
    }

    if !resp.status().is_success() {
        let text = resp.text().await?;
        bail!("Failed to fetch Minecraft profile: {text}");
    }

    Ok(resp.json().await?)
}

/// Full Microsoft → Xbox → XSTS → Minecraft auth chain
pub async fn full_auth_from_code(
    client: &reqwest::Client,
    code: &str,
    verifier: &str,
) -> Result<Account> {
    log::info!("Exchanging MS auth code...");
    let ms_tokens = exchange_code(client, code, verifier).await?;

    authenticate_with_ms_token(client, &ms_tokens).await
}

pub async fn authenticate_with_ms_token(
    client: &reqwest::Client,
    ms_tokens: &MsTokenResponse,
) -> Result<Account> {
    log::info!("Authenticating with Xbox Live...");
    let xbl = authenticate_xbl(client, &ms_tokens.access_token).await?;
    let uhs = extract_uhs(&xbl)?;

    log::info!("Getting XSTS token...");
    let xsts = authenticate_xsts(client, &xbl.token).await?;

    log::info!("Authenticating with Minecraft...");
    let mc_auth = authenticate_minecraft(client, &uhs, &xsts.token).await?;

    log::info!("Fetching Minecraft profile...");
    let profile = fetch_minecraft_profile(client, &mc_auth.access_token).await?;

    let now = chrono::Utc::now().timestamp();
    let expiry = now + ms_tokens.expires_in.unwrap_or(3600);

    Ok(Account {
        id: uuid::Uuid::new_v4().to_string(),
        username: profile.name,
        uuid: format_uuid(&profile.id),
        account_type: AccountType::Microsoft,
        access_token: Some(mc_auth.access_token),
        refresh_token: ms_tokens.refresh_token.clone(),
        token_expiry: Some(expiry),
        xuid: Some(extract_xuid_from_xsts(&xsts)),
        profile_icon: None,
    })
}

pub async fn refresh_account(
    client: &reqwest::Client,
    account: &mut crate::auth::accounts::Account,
) -> Result<()> {
    let refresh_token = account.refresh_token.as_deref()
        .context("No refresh token available")?;

    let ms_tokens = refresh_ms_token(client, refresh_token).await?;
    let refreshed = authenticate_with_ms_token(client, &ms_tokens).await?;

    account.access_token = refreshed.access_token;
    account.refresh_token = refreshed.refresh_token;
    account.token_expiry = refreshed.token_expiry;

    Ok(())
}

fn format_uuid(id: &str) -> String {
    if id.len() == 32 {
        format!(
            "{}-{}-{}-{}-{}",
            &id[..8], &id[8..12], &id[12..16], &id[16..20], &id[20..]
        )
    } else {
        id.to_string()
    }
}

fn extract_xuid_from_xsts(xsts: &XstsResponse) -> String {
    xsts.display_claims
        .get("xui")
        .and_then(|xui| xui.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("xid"))
        .and_then(|xid| xid.as_str())
        .unwrap_or("")
        .to_string()
}
