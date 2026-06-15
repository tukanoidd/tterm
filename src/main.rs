pub mod app;
pub mod config;
pub mod multiplex;

use rootcause::{Result, hooks::Hooks};
use rootcause_backtrace::BacktraceCollector;
use tracing_subscriber::prelude::*;

use crate::app::App;

fn main() -> Result<()> {
    init_rootcause()?;
    init_tracing()?;

    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .run()?;

    Ok(())
}

fn init_rootcause() -> Result<()> {
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install()?;

    Ok(())
}

fn init_tracing() -> Result<()> {
    let level = match cfg!(debug_assertions) {
        true => "debug",
        false => "info",
    };

    const EXTERNAL_LEVELS: &[(&str, &[&str])] = &[
        ("error", &["wgpu_hal"]),
        (
            "warn",
            &[
                "naga",
                "sctk",
                "wgpu_core",
                "cosmic_text",
                "iced_wgpu",
                "iced_winit",
            ],
        ),
    ];

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(tracing_subscriber::EnvFilter::new(format!(
            "{level},{}",
            EXTERNAL_LEVELS
                .iter()
                .flat_map(|(level, crates)| crates
                    .iter()
                    .map(move |crate_| format!("{crate_}={level}")))
                .collect::<Vec<_>>()
                .join(",")
        )))
        .try_init()?;

    tracing::debug!("Tracing initialized!");

    Ok(())
}
