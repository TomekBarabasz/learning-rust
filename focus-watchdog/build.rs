fn main() {
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