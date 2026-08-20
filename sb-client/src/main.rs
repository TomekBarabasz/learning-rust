//! sbcall — wywołuje funkcję Space Lua na serwerze SilverBullet przez Runtime API.
//!
//!   sbcall planner.next praca
//!   sbcall --url https://sb.example.com --token XXX planner.next praca 3

use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde_json::Value;

#[derive(Parser, Debug)]
#[command(
    name = "sbcall",
    about = "Wywołuje funkcję Space Lua na serwerze SilverBullet (Runtime API)"
)]
struct Args {
    /// Nazwa funkcji, np. planner.next (pomijana przy --eval)
    function: Option<String>,

    /// Argumenty funkcji. Liczby, true/false i nil idą jako wartości Lua,
    /// reszta jako stringi. Prefiks 'str:' wymusza string, 'lua:' surowy kod.
    args: Vec<String>,

    /// Surowe wyrażenie Lua zamiast wywołania funkcji, np. -e 'type(planner.next)'.
    /// Użyj '-' żeby czytać ze stdin.
    #[arg(short = 'e', long, conflicts_with_all = ["function", "args"])]
    eval: Option<String>,

    /// Wczytaj kod z pliku (implikuje --script)
    #[arg(short = 'f', long, conflicts_with_all = ["function", "args", "eval"])]
    file: Option<std::path::PathBuf>,

    /// Wyślij jako skrypt (endpoint lua_script) zamiast pojedynczego wyrażenia.
    /// Skrypt musi zwracać wynik przez `return`.
    #[arg(short = 's', long)]
    script: bool,

    /// Adres serwera SilverBullet
    #[arg(short, long, env = "SB_URL", default_value = "http://localhost:3000")]
    url: String,

    /// Token z SB_AUTH_TOKEN serwera (Authorization: Bearer ...)
    #[arg(short, long, env = "SB_TOKEN")]
    token: Option<String>,

    /// Timeout wykonania po stronie serwera, w sekundach
    #[arg(long, default_value_t = 30)]
    timeout: u64,

    /// Wypisz surową odpowiedź serwera zamiast samego pola "result"
    #[arg(long)]
    raw: bool,

    /// Nie formatuj JSON-a (jedna linia — wygodne do potokowania do jq)
    #[arg(short, long)]
    compact: bool,

    /// Pokaż wysyłane wyrażenie Lua i nic nie wywołuj
    #[arg(long)]
    dry_run: bool,
}

/// Zamienia argument z linii poleceń na literał Lua.
fn to_lua_literal(arg: &str) -> String {
    if let Some(rest) = arg.strip_prefix("lua:") {
        return rest.to_string();
    }
    if let Some(rest) = arg.strip_prefix("str:") {
        // serde_json escapuje tak samo jak Lua dla zwykłych stringów
        return Value::String(rest.to_string()).to_string();
    }
    match arg {
        "true" | "false" | "nil" => arg.to_string(),
        _ => {
            if arg.parse::<f64>().is_ok() {
                arg.to_string()
            } else {
                Value::String(arg.to_string()).to_string()
            }
        }
    }
}

fn build_expression(function: &str, args: &[String]) -> String {
    let inner: Vec<String> = args.iter().map(|a| to_lua_literal(a)).collect();
    format!("{}({})", function, inner.join(", "))
}

fn main() -> Result<()> {
    let args = Args::parse();

    let expression = match (&args.file, &args.eval, &args.function) {
        (Some(path), _, _) => std::fs::read_to_string(path)
            .with_context(|| format!("nie udało się przeczytać {}", path.display()))?
            .trim()
            .to_string(),
        (None, Some(e), _) if e == "-" => {
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
                .context("nie udało się przeczytać stdin")?;
            buf.trim().to_string()
        }
        (None, Some(e), _) => e.clone(),
        (None, None, Some(f)) => build_expression(f, &args.args),
        (None, None, None) => {
            bail!("podaj nazwę funkcji, wyrażenie (-e) albo plik ze skryptem (-f)")
        }
    };
    if args.dry_run {
        println!("{expression}");
        return Ok(());
    }

    // plik zwykle zawiera definicje + return, więc domyślnie tryb skryptu
    let script_mode = args.script || args.file.is_some();
    let path = if script_mode { "/.runtime/lua_script" } else { "/.runtime/lua" };
    let endpoint = format!("{}{}", args.url.trim_end_matches('/'), path);

    let client = reqwest::blocking::Client::builder()
        // margines nad timeoutem serwera: pierwsze wywołanie budzi headless Chrome
        .timeout(Duration::from_secs(args.timeout + 15))
        .build()?;

    let mut req = client
        .post(&endpoint)
        .header("X-Timeout", args.timeout.to_string())
        .header("Content-Type", "text/plain")
        .body(expression);

    if let Some(token) = &args.token {
        req = req.bearer_auth(token);
    }

    let resp = req
        .send()
        .with_context(|| format!("nie udało się połączyć z {endpoint}"))?;

    let status = resp.status();
    let text = resp.text().context("nie udało się odczytać odpowiedzi")?;

    let body: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => {
            // np. strona logowania w HTML zamiast JSON-a
            let preview: String = text.chars().take(200).collect();
            bail!("serwer zwrócił {status}, a treść nie jest JSON-em:\n{preview}");
        }
    };

    if !status.is_success() {
        let msg = body["error"].as_str().unwrap_or("nieznany błąd");
        let code = body["code"].as_str().unwrap_or("-");
        let hint = match status.as_u16() {
            401 | 403 => "  (sprawdź --token / SB_AUTH_TOKEN na serwerze)",
            503 => "  (Runtime API wyłączone albo headless Chrome jeszcze nie wstał)",
            504 => "  (podbij --timeout)",
            500 => "  (błąd w samym kodzie Lua)",
            _ => "",
        };
        bail!("{status} [{code}]: {msg}{hint}");
    }

    let out = if args.raw { &body } else { &body["result"] };
    if args.compact {
        println!("{out}");
    } else {
        println!("{}", serde_json::to_string_pretty(out)?);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals() {
        assert_eq!(to_lua_literal("praca"), "\"praca\"");
        assert_eq!(to_lua_literal("42"), "42");
        assert_eq!(to_lua_literal("true"), "true");
        assert_eq!(to_lua_literal("nil"), "nil");
        assert_eq!(to_lua_literal("str:42"), "\"42\"");
        assert_eq!(to_lua_literal("lua:{a=1}"), "{a=1}");
        // cudzysłowy i backslashe nie mogą uciec z literału
        assert_eq!(to_lua_literal(r#"a" or true--"#), r#""a\" or true--""#);
    }

    #[test]
    fn expressions() {
        assert_eq!(build_expression("planner.next", &[]), "planner.next()");
        assert_eq!(
            build_expression("planner.next", &["praca".into(), "3".into()]),
            "planner.next(\"praca\", 3)"
        );
    }
}