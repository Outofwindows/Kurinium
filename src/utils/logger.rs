use std::io::Write;
use std::sync::OnceLock;
use tracing_subscriber::{EnvFilter, fmt::MakeWriter};

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub struct Logger;
impl Logger {
    fn new() -> Self { Self }
}

impl<'a> MakeWriter<'a> for Logger {
    type Writer = std::io::Stdout;
    fn make_writer(&'a self) -> Self::Writer {
        std::io::stdout()
    }
}

fn parse_directive(s: &str) -> tracing_subscriber::filter::Directive {
    s.parse().unwrap_or_else(|e| {
        eprintln!("Failed to parse log directive '{}': {}", s, e);
        "warn".parse().expect("default 'warn' directive must be valid")
    })
}

pub fn init_logger() {
    LOGGER.get_or_init(Logger::new);
    let filter = EnvFilter::new("")
        .add_directive(parse_directive("hyper=error"))
        .add_directive(parse_directive("h2=error"))
        .add_directive(parse_directive("rustls=error"))
        .add_directive(parse_directive("reqwest=error"))
        .add_directive(parse_directive("tungstenite=error"))
        .add_directive(parse_directive("tokio_tungstenite=error"))
        .add_directive(parse_directive("serenity=error"))
        .add_directive(parse_directive("poise=error"))
        .add_directive(parse_directive("tracing=error"))
        .add_directive(parse_directive("kurinium=debug"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .with_target(false)
        .with_ansi(true)
        .init();
}

pub fn log(message: &str) {
    if let Some(_) = LOGGER.get() {
        let mut stdout = std::io::stdout();
        let _ = writeln!(stdout, "[DEBUG] {}", message);
    }
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            if $crate::config::Config::SHOW_CONSOLE {
                $crate::utils::logger::log(&format!($($arg)*));
            }
        }
    };
}
