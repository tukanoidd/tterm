pub mod app;
pub mod cli;
pub mod config;
pub mod multiplex;

use std::sync::OnceLock;

use clap::Parser;
use iced::advanced::graphics::core::window;
use iced_fonts::LUCIDE_FONT_BYTES;
use rootcause::{Result, hooks::Hooks};
use rootcause_backtrace::BacktraceCollector;
use tracing_subscriber::prelude::*;
use tterm_macros::fonts;

use crate::{
    app::App,
    cli::{Cli, LogLevel},
    config::Config,
};

fonts!("assets/fonts/");

static CLI_PRESET: OnceLock<Option<String>> = OnceLock::new();

fn main() -> Result<()> {
    init_rootcause()?;

    let Cli {
        preset,
        print_default_config,
        log_level,
    } = Cli::parse();
    let _ = CLI_PRESET.set(preset);

    init_tracing(log_level)?;

    if print_default_config {
        println!(
            "{}",
            Config::ron_options()
                .to_string_pretty(&Config::default(), Config::ron_pretty_config())?
        );

        return Ok(());
    }

    let app = iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .font(LUCIDE_FONT_BYTES)
        .window(window::Settings {
            maximized: true,
            visible: true,
            resizable: true,
            closeable: true,
            minimizable: true,
            decorations: false,
            transparent: true,
            blur: true,
            level: window::Level::Normal,
            exit_on_close_request: true,
            position: window::Position::Centered,
            ..Default::default()
        });
    let app = fonts::load(app).default_font(fonts::MONOSPACE_ROBOTO_MONO_NERD_FONT_MONO_FONT);

    app.run()?;

    Ok(())
}

fn init_rootcause() -> Result<()> {
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install()?;

    Ok(())
}

fn init_tracing(log_level: LogLevel) -> Result<()> {
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
            "{log_level},{}",
            EXTERNAL_LEVELS
                .iter()
                .flat_map(|(level, crates)| crates
                    .iter()
                    .map(move |crate_| format!("{crate_}={level}")))
                .collect::<Vec<_>>()
                .join(",")
        )))
        .try_init()?;

    tracing::debug!("Tracing initialized! [{log_level}]");

    Ok(())
}
