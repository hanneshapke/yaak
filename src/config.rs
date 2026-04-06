use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

pub fn load_config() -> Config {
    let mut candidates = Vec::new();
    // XDG-style ~/.config (works on all platforms)
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".config").join("yaak").join("config.toml"));
    }
    // Platform-native config dir (e.g. ~/Library/Application Support on macOS)
    if let Some(config_dir) = dirs::config_dir() {
        candidates.push(config_dir.join("yaak").join("config.toml"));
    }
    for path in candidates {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(config) = toml::from_str(&contents) {
                return config;
            }
        }
    }
    Config::default()
}

pub fn config_path() -> std::path::PathBuf {
    if let Some(home) = dirs::home_dir() {
        let xdg_path = home.join(".config").join("yaak").join("config.toml");
        if xdg_path.exists() {
            return xdg_path;
        }
    }
    if let Some(config_dir) = dirs::config_dir() {
        let native_path = config_dir.join("yaak").join("config.toml");
        if native_path.exists() {
            return native_path;
        }
    }
    // Default: prefer ~/.config on Unix, platform-native elsewhere
    if cfg!(unix) {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".config")
            .join("yaak")
            .join("config.toml")
    } else {
        dirs::config_dir()
            .unwrap_or_default()
            .join("yaak")
            .join("config.toml")
    }
}

pub fn resolve(cli: Option<String>, config: Option<String>, fallback: &str) -> String {
    cli.or(config).unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_prefers_cli() {
        assert_eq!(
            resolve(Some("cli".into()), Some("config".into()), "fallback"),
            "cli"
        );
    }

    #[test]
    fn resolve_falls_back_to_config() {
        assert_eq!(resolve(None, Some("config".into()), "fallback"), "config");
    }

    #[test]
    fn resolve_falls_back_to_default() {
        assert_eq!(resolve(None, None, "fallback"), "fallback");
    }
}
