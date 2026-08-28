
use std::path::PathBuf;
use crate::egui;

/// Animacja wczytana do pamięci albo informacja, czemu się nie udało.
#[derive(Clone)]
pub struct Anim {
    /// URI w schemacie `bytes://`, pod którym bajty siedzą w cache egui.
    pub uri: String,
    /// Komunikat do pokazania zamiast obrazka.
    pub error: Option<String>,
}

// Animacje wkompilowane w binarkę na etapie 'cargo build'.

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

/// Ikona okna (pasek zadań, Alt+Tab). Kwadratowy PNG, najlepiej 256x256.
const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Wczytuje animację o podanej nazwie bazowej.
///
/// Kolejność: najpierw plik z dysku (obok exe, potem w katalogu roboczym,
/// rozszerzenia .webp / .gif / .png), a jeśli go nie ma - wersja wkompilowana
/// w binarkę. Dzięki temu domyślnie wystarczy sam exe, ale można podmienić
/// animację bez rekompilacji, wrzucając plik do `assets/` obok niego.
///
/// # Panics
///
/// Funkcja nie panikuje.
///
/// Bajty trafiają wprost do cache egui pod URI `bytes://nazwa.rozszerzenie`.
/// Dzięki temu omijamy loader `file://` i całą zabawę ze ścieżkami na Windows -
/// plik czytamy sami i od razu wiemy, czy się udało.
pub fn load_anim(ctx: &egui::Context, stem: &str) -> Anim {
    const EXTENSIONS: [&str; 3] = ["webp", "gif", "png"];

    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.join("assets"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("assets"));
    }

    let mut tried: Vec<String> = Vec::new();

    for dir in &dirs {
        for ext in EXTENSIONS {
            let path = dir.join(format!("{stem}.{ext}"));
            if !path.is_file() {
                tried.push(path.display().to_string());
                continue;
            }
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let uri = format!("bytes://{stem}.{ext}");
                    log::info!(
                        "Wczytano {} ({} bajtów) jako {uri}",
                        path.display(),
                        bytes.len()
                    );
                    ctx.include_bytes(uri.clone(), bytes);
                    return Anim { uri, error: None };
                }
                Err(err) => {
                    log::error!("Nie mogę przeczytać {}: {err}", path.display());
                    tried.push(format!("{} ({err})", path.display()));
                }
            }
        }
    }

    // Nic na dysku - użyj wersji wkompilowanej w binarkę.
    for (name, bytes) in EMBEDDED {
        if *name == stem {
            let uri = format!("bytes://{stem}");
            log::info!("Używam wbudowanej animacji {uri} ({} bajtów)", bytes.len());
            ctx.include_bytes(uri.clone(), *bytes);
            return Anim { uri, error: None };
        }
    }

    let msg = format!("Brak pliku {stem}.(webp|gif|png).\nSzukałem w:\n{}", tried.join("\n"));
    log::error!("{msg}");
    Anim {
        uri: String::new(),
        error: Some(msg),
    }
}

/// Dekoduje wbudowany PNG do formatu, którego oczekuje egui.
/// Zwraca None zamiast panikować - brak ikony to nie powód, żeby apka nie wstała.
pub fn load_icon() -> Option<egui::IconData> {
    match image::load_from_memory(ICON_PNG) {
        Ok(img) => {
            let img = img.into_rgba8();
            let (width, height) = img.dimensions();
            Some(egui::IconData {
                rgba: img.into_raw(),
                width,
                height,
            })
        }
        Err(err) => {
            log::error!("Nie mogę zdekodować ikony: {err}");
            None
        }
    }
}
