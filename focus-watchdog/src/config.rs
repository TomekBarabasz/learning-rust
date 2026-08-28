use serde::{Deserialize, Serialize};
use std::{fs, path::{PathBuf}};
use toml;
use std::env;
use anyhow::Result;


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub url: Option<String>,
    pub token: Option<String>,
    pub worklog_file: Option<String>,
    pub window_size: (f32, f32),
    pub button_height: f32,
    pub text_size: f32,
    /// Czy w trybie working natychmiast przywracać okno, gdy jednak zostanie
    /// zminimalizowane (np. przez Win+D albo Win+M, których przycisk nie blokuje).
    /// Domyślnie wyłączone - walczenie z użytkownikiem o okno bywa irytujące.
    pub force_restore: bool,
    pub anim_size: f32,
    pub extend_options: Vec<u32>,
    pub overtime_color: (u8, u8, u8),
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            url: None,
            token: None,
            window_size: (640.0, 240.0),
            worklog_file: None,
            button_height: 28.0,
            text_size: 20.0,
            force_restore: false,
            anim_size: 200.0,
            extend_options: vec![15, 30, 45],
            overtime_color: (46, 160, 67),
        }
    }
}

fn get_default_config_path() -> Result<PathBuf> {
    Ok(env::current_exe()?.with_extension("toml"))
}

pub fn load_config(path: Option<&str>) -> Option<AppConfig> {
    let path = match path {
        Some(p) => PathBuf::from(p),
        None => get_default_config_path().ok()?,
    };
    let config_str = fs::read_to_string(&path).ok()?;
    let config: AppConfig = toml::from_str(&config_str).ok()?;
    log::info!("AppConfig loaded correctly");
    Some(config)
}
