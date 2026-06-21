pub mod app;
pub mod config;
pub mod multiplex;

use iced::advanced::graphics::core::window;
use iced_fonts::LUCIDE_FONT_BYTES;
use rootcause::{Result, hooks::Hooks};
use rootcause_backtrace::BacktraceCollector;
use tracing_subscriber::prelude::*;
use tterm_macros::fonts;

use crate::app::App;

fonts!("assets/fonts/");

fn main() -> Result<()> {
    init_rootcause()?;
    init_tracing()?;

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
    let app = fonts::load(app);

    app.run()?;

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
