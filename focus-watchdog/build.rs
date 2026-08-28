//! Przygotowanie zasobów przed kompilacją.
//!
//! Oryginalne animacje leżą w `assets/anim` w pełnej rozdzielczości.
//! Do exe trafiają wersje przeskalowane i przerzedzone przez ffmpeg -
//! każda klatka GIF-a to osobna tekstura w pamięci karty, więc rozmiar
//! pliku źródłowego przekłada się wprost na zużycie VRAM.
//!
//! Brak ffmpeg nie psuje builda: pliki są wtedy wkompilowane bez zmian.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Maksymalny bok wkompilowanej animacji w pikselach.
/// Z zapasem na skalowanie DPI - patrz komentarz w README.
const EMBED_SIZE: u32 = 200;

/// Liczba klatek na sekundę po konwersji.
const EMBED_FPS: u32 = 15;

/// Katalog z oryginałami.
const SOURCE_DIR: &str = "assets";

/// Animacja gotowa do wkompilowania.
struct Animation {
    /// Nazwa pliku z rozszerzeniem - po nim egui rozpoznaje format.
    file_name: String,
    /// Ścieżka w OUT_DIR.
    path: PathBuf,
}

fn main() {
    println!("cargo:rerun-if-changed={SOURCE_DIR}");
    println!("cargo:rerun-if-env-changed=NOPE_FFMPEG");

    let animations = prepare_animations();
    write_embedded_table(&animations);
    embed_windows_icon();
}

// ------------------------------------------------------------ animacje ---

fn prepare_animations() -> Vec<Animation> {
    let target_dir = converted_dir();
    fs::create_dir_all(&target_dir).expect("nie mogę utworzyć katalogu w OUT_DIR");

    let ffmpeg = find_ffmpeg();
    let mut animations = Vec::new();

    for source in source_animations() {
        let file_name = source
            .file_name()
            .expect("plik bez nazwy")
            .to_string_lossy()
            .into_owned();
        let target = target_dir.join(&file_name);

        if !is_up_to_date(&source, &target) {
            convert_or_copy(ffmpeg.as_deref(), &source, &target);
        }

        animations.push(Animation {
            file_name,
            path: target,
        });
    }

    animations
}

/// Parametry konwersji siedzą w nazwie katalogu, więc ich zmiana
/// automatycznie unieważnia wszystko, co zostało z poprzedniego builda.
fn converted_dir() -> PathBuf {
    let out_dir = env::var("OUT_DIR").expect("cargo nie ustawiło OUT_DIR");
    Path::new(&out_dir).join(format!("anim-{EMBED_SIZE}px-{EMBED_FPS}fps"))
}

fn source_animations() -> Vec<PathBuf> {
    let dir = match fs::read_dir(SOURCE_DIR) {
        Ok(dir) => dir,
        Err(err) => panic!("nie mogę czytać {SOURCE_DIR}: {err}"),
    };

    let mut files: Vec<PathBuf> = dir
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| is_animation(path))
        .collect();

    // Kolejność w tablicy EMBEDDED nie może zależeć od systemu plików.
    files.sort();
    files
}

fn is_animation(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("gif") | Some("webp")
    )
}

/// Wynik jest aktualny, jeśli istnieje i nie jest starszy od źródła.
fn is_up_to_date(source: &Path, target: &Path) -> bool {
    let (Ok(src), Ok(dst)) = (fs::metadata(source), fs::metadata(target)) else {
        return false;
    };
    let (Ok(src), Ok(dst)) = (src.modified(), dst.modified()) else {
        return false;
    };

    dst >= src
}

fn convert_or_copy(ffmpeg: Option<&Path>, source: &Path, target: &Path) {
    match ffmpeg {
        Some(binary) => shrink_animation(binary, source, target),
        None => {
            println!(
                "cargo:warning=ffmpeg niedostępny - {} zostaje w oryginalnym rozmiarze",
                source.display()
            );
            fs::copy(source, target).expect("nie mogę skopiować animacji");
        }
    }
}

/// Skalowanie i przerzedzenie klatek, potem paleta liczona z tego,
/// co faktycznie trafi do wyniku. Kolejność filtrów jest istotna:
/// palettegen po fps i scale, inaczej paleta nie pasuje do obrazu.
fn shrink_animation(ffmpeg: &Path, source: &Path, target: &Path) {
    let filter = format!(
        "fps={EMBED_FPS},\
         scale={EMBED_SIZE}:{EMBED_SIZE}:force_original_aspect_ratio=decrease:flags=lanczos,\
         split[a][b];\
         [a]palettegen=stats_mode=diff:reserve_transparent=1[p];\
         [b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle:alpha_threshold=128"
    );

    let status = Command::new(ffmpeg)
        .args(["-y", "-v", "error", "-i"])
        .arg(source)
        .args(["-vf", &filter, "-loop", "0"])
        .arg(target)
        .status()
        .expect("nie udało się uruchomić ffmpeg");

    assert!(
        status.success(),
        "ffmpeg zwrócił błąd dla {}",
        source.display()
    );
}

/// Ścieżka z `NOPE_FFMPEG` albo binarka z PATH. None = konwersja pominięta.
fn find_ffmpeg() -> Option<PathBuf> {
    let candidate = env::var("NOPE_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ffmpeg"));

    let works = Command::new(&candidate)
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    works.then_some(candidate)
}

// ------------------------------------------------- wygenerowany moduł ---

/// Wypisuje tablicę EMBEDDED do OUT_DIR. Dzięki temu dodanie animacji
/// to wrzucenie pliku do assets, bez dotykania kodu.
fn write_embedded_table(animations: &[Animation]) {
    let mut code = String::from(
        "// Generowane przez build.rs przy każdej kompilacji - nie edytować.\n\
         pub static EMBEDDED: &[(&str, &[u8])] = &[\n",
    );

    for anim in animations {
        code.push_str(&format!(
            "    (\"{}\", include_bytes!(\"{}\")),\n",
            anim.file_name,
            literal_path(&anim.path)
        ));
    }

    code.push_str("];\n");

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("embedded.rs");
    fs::write(path, code).expect("nie mogę zapisać embedded.rs");
}

/// include_bytes! przyjmuje literał, a backslashe z Windows byłyby w nim
/// sekwencjami ucieczki. Ukośniki działają na obu systemach.
fn literal_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

// ---------------------------------------------------------------- ikona ---

fn embed_windows_icon() {
    // Ikona samego pliku .exe - tylko Windows, tylko jeśli plik istnieje.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");

        if std::path::Path::new("assets/icon.ico").exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/icon.ico");
            if let Err(err) = res.compile() {
                println!("cargo:warning=Nie udało się wkompilować ikony: {err}");
            }
        } else {
            println!("cargo:warning=Brak assets/icon.ico - exe zostanie bez ikony");
        }
    }
}