use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("GLauncher")
}

pub fn instances_dir() -> PathBuf {
    data_dir().join("instances")
}

pub fn java_dir() -> PathBuf {
    data_dir().join("java")
}

pub fn assets_dir() -> PathBuf {
    data_dir().join("assets")
}

pub fn libraries_dir() -> PathBuf {
    data_dir().join("libraries")
}

pub fn versions_dir() -> PathBuf {
    data_dir().join("versions")
}

pub fn config_file() -> PathBuf {
    data_dir().join("config.json")
}

pub fn accounts_file() -> PathBuf {
    data_dir().join("accounts.json")
}

pub fn ensure_dirs() -> anyhow::Result<()> {
    for dir in &[
        data_dir(),
        instances_dir(),
        java_dir(),
        assets_dir(),
        libraries_dir(),
        versions_dir(),
    ] {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}
