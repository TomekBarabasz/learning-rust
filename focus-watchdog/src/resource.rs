use std::path::{Path, PathBuf};
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
// Tablica EMBEDDED trzyma pary (nazwa_pliku_z_rozszerzeniem, bajty).

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

/// Ikona okna (pasek zadań, Alt+Tab). Kwadratowy PNG, najlepiej 256x256.
const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Rozszerzenia sprawdzane na dysku, w kolejności preferencji.
const EXTENSIONS: [&str; 3] = ["webp", "gif", "png"];

/// Wczytuje animację o podanej nazwie bazowej.
///
/// Kolejność: najpierw plik z dysku (obok exe, potem w katalogu roboczym),
/// a jeśli go nie ma - wersja wkompilowana w binarkę. Dzięki temu domyślnie
/// wystarczy sam exe, ale można podmienić animację bez rekompilacji,
/// wrzucając plik do `assets/` obok niego.
///
/// # Panics
///
/// Funkcja nie panikuje.
pub fn load_anim(ctx: &egui::Context, stem: &str) -> Anim {
    let mut tried = Vec::new();

    if let Some(anim) = load_from_disk(ctx, stem, &mut tried) {
        return anim;
    }

    if let Some(anim) = load_embedded(ctx, stem) {
        return anim;
    }

    missing_anim(stem, &tried)
}

fn load_from_disk(ctx: &egui::Context, stem: &str, tried: &mut Vec<String>) -> Option<Anim> {
    for dir in asset_dirs() {
        for ext in EXTENSIONS {
            let path = dir.join(format!("{stem}.{ext}"));

            match read_candidate(&path) {
                Ok(Some(bytes)) => {
                    log::info!("Wczytano {} ({} bajtów)", path.display(), bytes.len());
                    return Some(publish_bytes(ctx, &format!("{stem}.{ext}"), bytes));
                }
                Ok(None) => tried.push(path.display().to_string()),
                Err(err) => {
                    log::error!("Nie mogę przeczytać {}: {err}", path.display());
                    tried.push(format!("{} ({err})", path.display()));
                }
            }
        }
    }

    None
}

/// Katalogi z animacjami: obok exe i w katalogu roboczym.
/// Bardzo często to ten sam folder - bez dedup log robi się dwa razy dłuższy.
fn asset_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.join("assets"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("assets"));
    }

    dirs.dedup();
    dirs
}

/// `Ok(None)` = pliku po prostu nie ma, `Err` = jest, ale nie da się przeczytać.
fn read_candidate(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    if !path.is_file() {
        return Ok(None);
    }

    std::fs::read(path).map(Some)
}

fn load_embedded(ctx: &egui::Context, stem: &str) -> Option<Anim> {
    let (file_name, bytes) = find_embedded(stem)?;

    log::info!(
        "Używam wbudowanej animacji {file_name} ({} bajtów)",
        bytes.len()
    );

    Some(publish_bytes(ctx, file_name, bytes))
}

/// EMBEDDED trzyma nazwy z rozszerzeniem ("idle.gif"), a wołamy po stemie
/// ("idle") - stąd porównanie po samej nazwie bazowej.
fn find_embedded(stem: &str) -> Option<(&'static str, &'static [u8])> {
    EMBEDDED
        .iter()
        .copied()
        .find(|(file_name, _)| file_stem_of(file_name) == stem)
}

fn file_stem_of(file_name: &str) -> &str {
    file_name.rsplit_once('.').map_or(file_name, |(stem, _)| stem)
}

/// Bajty trafiają wprost do cache egui pod URI `bytes://nazwa.rozszerzenie`.
/// Rozszerzenie w URI jest obowiązkowe - po nim egui dobiera dekoder.
///
/// `impl Into<Bytes>` pozwala oddać zarówno `Vec<u8>` z dysku, jak i
/// `&'static [u8]` z sekcji danych binarki - to drugie bez kopiowania.
fn publish_bytes(
    ctx: &egui::Context,
    file_name: &str,
    bytes: impl Into<egui::load::Bytes>,
) -> Anim {
    let uri = format!("bytes://{file_name}");
    ctx.include_bytes(uri.clone(), bytes);

    Anim { uri, error: None }
}

fn missing_anim(stem: &str, tried: &[String]) -> Anim {
    let msg = format!(
        "Brak pliku {stem}.(webp|gif|png).\nSzukałem w:\n{}",
        tried.join("\n")
    );
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