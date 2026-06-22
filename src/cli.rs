use clap::{Parser, ValueEnum};
use derive_more::Display;

/// Terminal emulator with multiplexing built-in
#[derive(Parser)]
pub struct Cli {
    #[arg(short, long)]
    pub preset: Option<String>,

    #[arg(short, long, value_enum, default_value_t = LogLevel::default())]
    pub log_level: LogLevel,
}

#[derive(Default, Debug, Display, Clone, Copy, ValueEnum)]
pub enum LogLevel {
    #[cfg_attr(not(debug_assertions), default)]
    Info,
    #[cfg_attr(all(debug_assertions, not(feature = "trace")), default)]
    Debug,
    #[cfg_attr(feature = "trace", default)]
    Trace,
}
