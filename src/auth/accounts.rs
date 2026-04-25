use serde::{Deserialize, Serialize};
use anyhow::Result;
use uuid::Uuid;
use crate::utils::paths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountType {
    Microsoft,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub account_type: AccountType,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expiry: Option<i64>,
    pub xuid: Option<String>,
    pub profile_icon: Option<String>,
}

impl Account {
    pub fn new_offline(username: &str) -> Self {
        // Offline accounts use a deterministic UUID based on username (Minecraft vanilla behaviour)
        let formatted_uuid = offline_uuid(username);
        Self {
            id: Uuid::new_v4().to_string(),
            username: username.to_string(),
            uuid: formatted_uuid,
            account_type: AccountType::Offline,
            access_token: Some("0".to_string()),
            refresh_token: None,
            token_expiry: None,
            xuid: None,
            profile_icon: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        if self.account_type == AccountType::Offline {
            return false;
        }
        if let Some(expiry) = self.token_expiry {
            let now = chrono::Utc::now().timestamp();
            return now >= expiry - 300; // 5 min buffer
        }
        true
    }

    pub fn display_type(&self) -> &str {
        match self.account_type {
            AccountType::Microsoft => "Microsoft",
            AccountType::Offline => "Offline",
        }
    }
}

/// Vanilla-compatible offline UUID: SHA-256 of "OfflinePlayer:<username>", formatted as UUID v3
fn offline_uuid(username: &str) -> String {
    use sha2::{Sha256, Digest};
    let input = format!("OfflinePlayer:{username}");
    let hash = Sha256::digest(input.as_bytes());
    // Take first 16 bytes and format as UUID
    let h = &hash[..16];
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        h[0], h[1], h[2], h[3],
        h[4], h[5],
        h[6], h[7],
        h[8], h[9],
        h[10], h[11], h[12], h[13], h[14], h[15]
    )
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AccountManager {
    pub accounts: Vec<Account>,
    pub active_account_id: Option<String>,
}

impl AccountManager {
    pub fn load() -> Result<Self> {
        let path = paths::accounts_file();
        if path.exists() {
            let s = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&s).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = paths::accounts_file();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let s = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, s)?;
        Ok(())
    }

    pub fn add_account(&mut self, account: Account) {
        let id = account.id.clone();
        // Remove existing with same uuid
        self.accounts.retain(|a| a.uuid != account.uuid);
        self.accounts.push(account);
        self.active_account_id = Some(id);
    }

    pub fn remove_account(&mut self, id: &str) {
        self.accounts.retain(|a| a.id != id);
        if self.active_account_id.as_deref() == Some(id) {
            self.active_account_id = self.accounts.first().map(|a| a.id.clone());
        }
    }

    pub fn active_account(&self) -> Option<&Account> {
        self.active_account_id.as_ref().and_then(|id| {
            self.accounts.iter().find(|a| &a.id == id)
        })
    }

    pub fn set_active(&mut self, id: &str) {
        if self.accounts.iter().any(|a| a.id == id) {
            self.active_account_id = Some(id.to_string());
        }
    }

    pub fn update_account(&mut self, account: Account) {
        if let Some(existing) = self.accounts.iter_mut().find(|a| a.id == account.id) {
            *existing = account;
        }
    }
}
