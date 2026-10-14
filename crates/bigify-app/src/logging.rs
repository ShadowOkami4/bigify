//! Logging to stderr and `$XDG_STATE_HOME/bigify/bigify.log`.
//!
//! Verbosity comes from `BIGIFY_LOG` (env_logger syntax, default `info`).
//! librespot modules that handle tokens or credentials log them at `trace`
//! (and the HTTP client logs URLs containing search queries), so they are
//! capped at `info` regardless of what the user requests.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};

use bigify_core::paths;
use log::LevelFilter;

const MAX_LOG_BYTES: u64 = 4 * 1024 * 1024;

/// Modules whose debug/trace output may contain secrets.
const SENSITIVE_MODULES: &[&str] = &[
    "librespot_oauth",
    "librespot_core::token",
    "librespot_core::login5",
    "librespot_core::authentication",
    "librespot_core::session",
    "librespot_core::connection",
    // Logs request URLs, which include search queries.
    "librespot_core::http_client",
    "librespot_discovery",
    "oauth2",
    "reqwest",
    "hyper",
    "hyper_util",
];

struct Tee {
    file: Option<File>,
}

impl Write for Tee {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _ = io::stderr().write_all(buf);
        if let Some(f) = self.file.as_mut() {
            let _ = f.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(f) = self.file.as_mut() {
            f.flush()?;
        }
        io::stderr().flush()
    }
}

fn open_log_file() -> Option<File> {
    let path = paths::log_file();
    fs::create_dir_all(path.parent()?).ok()?;
    // Keep one previous log for bug reports; start fresh when large.
    if fs::metadata(&path).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
        let _ = fs::rename(&path, path.with_extension("log.old"));
    }
    OpenOptions::new().create(true).append(true).open(path).ok()
}

pub fn init() {
    let mut builder = env_logger::Builder::new();
    builder.filter_level(LevelFilter::Info);
    if let Ok(spec) = std::env::var("BIGIFY_LOG") {
        builder.parse_filters(&spec);
    }
    for module in SENSITIVE_MODULES {
        builder.filter_module(module, LevelFilter::Info);
    }
    builder
        .format_timestamp_millis()
        .target(env_logger::Target::Pipe(Box::new(Tee {
            file: open_log_file(),
        })))
        .init();
}
