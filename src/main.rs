pub mod app;
pub mod config;
pub mod multiplex;

use iced_fonts::LUCIDE_FONT_BYTES;
use rootcause::{Result, hooks::Hooks};
use rootcause_backtrace::BacktraceCollector;
use tracing_subscriber::prelude::*;
use tterm_macros::fonts;

use crate::app::App;

fonts![
    IosevkaFixed("IosevkaFixed-34.6.3" => "IosevkaFixed" @ {
        Monospace
    }): {
        Normal,
    }
];

fn main() -> Result<()> {
    init_rootcause()?;
    init_tracing()?;

    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .font(LUCIDE_FONT_BYTES)
        .font(IOSEVKA_FIXED_NORMAL_BYTES)
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
    let level = match cfg!(feature = "trace") {
        true => "trace",
        false => match cfg!(debug_assertions) {
            true => "debug",
            false => "info",
        },
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
                "iced_graphics",
                "alacritty_terminal",
                "vte",
                "calloop",
                "zbus",
            ],
        ),
    ];

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().compact())
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

    tracing::debug!("Tracing initialized! [{level}]");

    Ok(())
}
