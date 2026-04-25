use std::sync::{Arc, Mutex};
use crate::auth::accounts::AccountManager;
use crate::minecraft::instances::InstanceManager;
use crate::utils::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub accounts: Arc<Mutex<AccountManager>>,
    pub instances: Arc<Mutex<InstanceManager>>,
    pub config: Arc<Mutex<AppConfig>>,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        crate::utils::paths::ensure_dirs()?;

        let accounts = AccountManager::load().unwrap_or_default();
        let instances = InstanceManager::load().unwrap_or_default();
        let config = AppConfig::load().unwrap_or_default();
        let http_client = crate::utils::download::build_http_client()?;

        Ok(Self {
            accounts: Arc::new(Mutex::new(accounts)),
            instances: Arc::new(Mutex::new(instances)),
            config: Arc::new(Mutex::new(config)),
            http_client,
        })
    }
}
