use anyhow::{bail,Result,Context};
use std::{fs::File, fs::OpenOptions, io::Write, path::PathBuf};
//use std::time::{Duration, Instant};
use chrono::Local;
use std::time::Duration;
use serde_json::Value;
use serde::Deserialize;

use crate::config::AppConfig;
use crate::task::Task;
use crate::utils::to_minutes;

#[derive(Debug, Deserialize)]
pub struct WorklogTask {
    pub name: String,
    #[serde(rename = "ref")]
    pub tag: String,
}

pub trait Worklog {
    fn get_name(&self) -> String;
    fn add_record(&mut self, task: &Task) -> Result<()>;
    fn get_recent_tasks(&self) -> Vec<WorklogTask>;
}

struct FileWorklog {
    path: PathBuf,
    file: File,
}

/// Nagłówek pisany tylko przy zakładaniu nowego (pustego) pliku.
const CSV_HEADER: &str = "started_at,ended_at,task,tag,minimum_min,worked_min,paused_min";

const DATE_FMT: &str = "%Y-%m-%d %H:%M:%S";

/// Otacza pole cudzysłowami, jeśli zawiera przecinek, cudzysłów albo nową linię.
/// Cudzysłowy w środku podwajamy - tak wymaga RFC 4180 i tak czyta to pandas.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

impl FileWorklog {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        if file.metadata()?.len() == 0 {
            writeln!(file, "{CSV_HEADER}")?;
        }

        Ok(Self {
            path,
            file,
        })
    }
}

impl Worklog for FileWorklog {
    fn get_name(&self) -> String {
        format!("Logowanie do pliku {:?}",self.path)
    }
    fn add_record(&mut self, task: &Task) -> Result<()> {
        use std::io::Write as _;

        let fresh = match std::fs::metadata(&self.path) {
            Ok(meta) => meta.len() == 0,
            Err(_) => true,
        };

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        if fresh {
            writeln!(file, "{CSV_HEADER}")?;
        }

        writeln!(
            file,
            "{},{},{},{},{},{},{}",
            task.started_at.format(DATE_FMT),
            Local::now().format(DATE_FMT),
            csv_field(&task.name),
            csv_field(&task.tag),
            to_minutes(task.minimum),
            to_minutes(task.worked()),
            to_minutes(task.paused_time()),
        )?;
        self.file.flush()?;
        Ok(())
    }

    fn get_recent_tasks(&self) -> Vec<WorklogTask> {
        // Implementacja pobierania nazw zadań z pliku
        vec![]
    }
}

struct RemoteWorklog {
    url: String,
    token: String,
    client: reqwest::blocking::Client,
}

impl RemoteWorklog {
    pub fn new(url: String, token: String) -> Self {
        Self { url, token, client: reqwest::blocking::Client::builder()
                                    .timeout(Duration::from_secs(15))
                                    .build()
                                    .expect("Failed to build HTTP client") 
        }
    }
    fn send_expression(&self, expr : String) -> Result<Value> {
        let endpoint = format!("{}/.runtime/lua",self.url.trim_end_matches('/'));
        let req = self.client
            .post(&endpoint)
            .header("Content-Type", "text/plain")
            .bearer_auth(&self.token)
            .body(expr);
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
                let error_msg = format!("serwer zwrócił {status}, a treść nie jest JSON-em:\n{preview}");
                log::error!("{}",error_msg);
                bail!(error_msg);
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
            let error_msg = format!("{status} [{code}]: {msg}{hint}");
            log::error!("{}",error_msg);
            bail!(error_msg);
        }
        body.get("result")
        .cloned()
        .context("odpowiedź serwera nie zawiera pola `result`")
    }

    /// Konwertuje pole "result" odpowiedzi na listę zadań.
    /// Akceptuje tablicę, pojedynczy obiekt oraz `{empty=true}` / null jako pustą listę.
    fn to_tasks(result: &Value) -> Result<Vec<WorklogTask>> {
        match result {
            Value::Null => Ok(Vec::new()),
            Value::Array(_) => serde_json::from_value(result.clone())
                .context("nieoczekiwany kształt listy zadań"),
            Value::Object(map) => {
                if map.get("empty").and_then(Value::as_bool) == Some(true) {
                    return Ok(Vec::new());
                }
                let one: WorklogTask = serde_json::from_value(result.clone())
                    .context("nieoczekiwany kształt zadania")?;
                Ok(vec![one])
            }
            other => anyhow::bail!("oczekiwano obiektu albo tablicy, dostano: {other}"),
        }
    }
}

impl Worklog for RemoteWorklog {
    fn get_name(&self) -> String {
        format!("Logowanie na serwer {}",self.url)
    }

    fn add_record(&mut self, task: &Task) -> Result<()> {
        // TODO: sprawdzić w sb-client jak to zformatować poprawnie żeby Lua się nie pluło
        let record = format!(
            "{},{},{},{},{},{},{}",
            task.started_at.format(DATE_FMT),
            Local::now().format(DATE_FMT),
            csv_field(&task.name),
            csv_field(&task.tag),
            to_minutes(task.minimum),
            to_minutes(task.worked()),
            to_minutes(task.paused_time()),
        );
        let expr = format!("worklog.addCsv({})", serde_json::Value::String(record));
        let _result = self.send_expression(expr);
        Ok(())
    }

    fn get_recent_tasks(&self) -> Vec<WorklogTask> {
        let expr = "mytask.getNextTasks(10)";
        let result = self.send_expression(expr.to_string());
        match result {
            Ok(value) => RemoteWorklog::to_tasks(&value).unwrap_or_default(),
            Err(_) => vec![],
        }
    }
}

struct DummyWorklog {
}

impl DummyWorklog {
    pub fn new() -> Self {
        Self {}
    }
}
impl Worklog for DummyWorklog {
    fn get_name(&self) -> String {
        format!("Logowanie wyłączone")
    }
    fn add_record(&mut self, _task: &Task) -> Result<()> {
        Ok(())
    }

    fn get_recent_tasks(&self) -> Vec<WorklogTask> {
        // Implementacja pobierania nazw zadań z pliku
        vec![]
    }
}

pub fn make_worklog(config : Option<AppConfig>) -> Result<Box<dyn Worklog>> {
    if let Some(cfg) = config {
        if let (Some(url), Some(token)) = (cfg.url, cfg.token) {
            log::info!("Using server worklog, url = {}",url);
            return Ok(Box::new(RemoteWorklog::new(url, token)));
        } else if let Some(worklog_file) = cfg.worklog_file {
            let path = PathBuf::from(worklog_file);
            log::info!("Using file worklog, path = {:?}",path.clone());
            let worklog = FileWorklog::new(path)?;
            return Ok(Box::new(worklog));
        }
    };
    return Ok(Box::new(DummyWorklog::new()));
}
