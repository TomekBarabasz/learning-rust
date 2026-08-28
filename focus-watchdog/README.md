# konfiguracja [optional]
w tym samym katalogu co focus-watchdog.exe
robimy plik focus-watchdog.toml
    url = "my silverbullet url"
    token = "my silverbuilet api token"
    worklog_file = "filename" [optional]
    window_size = [640,240] [optional]

# takie tam
## konwersja webp na gif [ręcznie]
`ffmpeg -i <file>.webp -loop 0 <file>.gif`

## podawanie ścieżki do ffmpeg
podczas builda animacje są konwertowane do rozmiaru EMBED_SIZE w build.rs
- windows:
    set NOPE_FFMPEG=C:\\tomek\\ffmpeg-2026-08-17\\bin\\ffmpeg.exe
- linux: .. ?
