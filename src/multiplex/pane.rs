use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

use bon::bon;
use derive_more::{Debug, From};
use iced::widget::{container, pane_grid};
use iced_term::{Terminal, TerminalView};
use rootcause::Result;
use uuid::Uuid;

static TERM_ID: AtomicU64 = AtomicU64::new(0);

use crate::{
    app::{AppElement, AppMsg, AppSubscription, AppTask},
    config::terminal::TerminalConfig,
};

#[derive(Debug)]
pub struct PaneState {
    pub id: Uuid,
    pub term_id: u64,

    #[debug(skip)]
    terminal: Terminal,
}

#[bon]
impl PaneState {
    #[builder]
    pub fn new(id: Uuid, terminal_config: &TerminalConfig) -> Result<Self> {
        let term_id = TERM_ID.load(Ordering::SeqCst);

        TERM_ID.store(term_id + 1, Ordering::SeqCst);

        let TerminalConfig { font, theme } = terminal_config;

        // TODO: configurable working_dir
        let working_directory = std::env::current_dir()?;
        let env = std::env::vars().collect::<HashMap<_, _>>();

        let terminal = Terminal::new(
            term_id,
            iced_term::settings::Settings {
                font: font.clone().into(),
                theme: theme.clone().into(),
                backend: iced_term::settings::BackendSettings {
                    // TODO: configurable program
                    program: "nu".into(),
                    args: vec![],
                    env,
                    working_directory: Some(working_directory),
                },
            },
        )?;

        Ok(Self {
            id,
            term_id,
            terminal,
        })
    }

    pub fn view(&self, is_focused: bool) -> AppElement<'_> {
        container(
            TerminalView::show(&self.terminal)
                .map(|e| IdPaneMessage {
                    id: self.id,
                    msg: e.into(),
                })
                .map(AppMsg::from),
        )
        .padding(4)
        .style(move |theme| {
            let palette = theme.extended_palette();

            let style = container::bordered_box(theme);
            style.border(style.border.color(match is_focused {
                true => palette.primary.strong.color,
                false => palette.secondary.base.color,
            }))
        })
        .into()
    }

    pub fn update(&mut self, msg: PaneMessage) -> AppTask {
        match msg {
            PaneMessage::TerminalMsg(iced_term::Event::BackendCall(_, command)) => {
                let action = self
                    .terminal
                    .handle(iced_term::Command::ProxyToBackend(command));

                if action == iced_term::actions::Action::Shutdown {
                    return AppTask::done(
                        IdPaneMessage {
                            id: self.id,
                            msg: PaneMessage::Close,
                        }
                        .into(),
                    );
                }
            }
            _ => unreachable!(),
        }

        AppTask::none()
    }

    pub fn subscription(&self) -> AppSubscription {
        let id = self.id;

        self.terminal
            .subscription()
            .with(id)
            .map(|(id, e)| IdPaneMessage { id, msg: e.into() })
            .map(AppMsg::from)
    }

    pub fn focus(&self) -> AppTask {
        TerminalView::focus(self.terminal.widget_id().clone())
    }
}

#[derive(Debug, Clone)]
pub struct IdPaneMessage {
    pub id: Uuid,
    pub msg: PaneMessage,
}

#[derive(Debug, Clone, From)]
pub enum PaneMessage {
    Resize(pane_grid::ResizeEvent),
    Dragged(pane_grid::DragEvent),
    Close,

    TerminalMsg(iced_term::Event),
}
