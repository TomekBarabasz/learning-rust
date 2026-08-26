use serde::{Deserialize, Serialize};
use std::{fs, path::{PathBuf}};
use toml;
use std::env;
use anyhow::Result;


#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub url: Option<String>,
    pub token: Option<String>,
    pub window_size: Option<(f32, f32)>,
    pub worklog_file: Option<String>,
    pub button_height: f32,
    pub text_size: f32,
    pub force_restore: bool,
    pub anim_size: f32,
    pub extend_options: Vec<u32>,
    pub overtime_color: (u8, u8, u8),
}

/// Wysokość przycisków i comboboxa w wierszu akcji.
const BUTTON_H: f32 = 28.0;

/// Wielkość czcionki w trybie working (domyślna w egui to ok. 14).
const TEXT_SIZE: f32 = 20.0;

/// Czy w trybie working natychmiast przywracać okno, gdy jednak zostanie
/// zminimalizowane (np. przez Win+D albo Win+M, których przycisk nie blokuje).
/// Domyślnie wyłączone - walczenie z użytkownikiem o okno bywa irytujące.
const FORCE_RESTORE: bool = false;

/// Bok kwadratowej animacji w pikselach.
const ANIM_SIZE: f32 = 200.0;

/// Warianty przedłużenia sesji, w minutach.
const EXTEND_OPTIONS: [u32; 3] = [15, 30, 45];

/// Kolor czasu po wypełnieniu minimum. Czytelny i na jasnym, i na ciemnym motywie.
const OVERTIME_COLOR: (u8, u8, u8) = (46, 160, 67);

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            url: None,
            token: None,
            window_size: None,
            worklog_file: None,
            button_height: BUTTON_H,
            text_size: TEXT_SIZE,
            force_restore: FORCE_RESTORE,
            anim_size: ANIM_SIZE,
            extend_options: EXTEND_OPTIONS.to_vec(),
            overtime_color: OVERTIME_COLOR,
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
    Some(config)
}
