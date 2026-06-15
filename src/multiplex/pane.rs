use std::collections::HashMap;

use bon::bon;
use derive_more::{Debug, From};
use iced_term::{Terminal, TerminalView};
use rootcause::Result;
use uuid::Uuid;

use crate::{
    app::{AppElement, AppMsg, AppSubscription, AppTask},
    config::TerminalConfig,
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
    pub fn new(id: Uuid, terminal_config: TerminalConfig) -> Result<Self> {
        let (id1, id2) = id.as_u64_pair();
        let term_id = id1.wrapping_add(id2);

        let TerminalConfig { font, theme } = terminal_config;

        // TODO: configurable working_dir
        let working_directory = std::env::current_dir()?;
        let env = std::env::vars().collect::<HashMap<_, _>>();

        let terminal = Terminal::new(
            term_id,
            iced_term::settings::Settings {
                font: font.into(),
                theme: theme.into(),
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

    pub fn view(&self) -> AppElement<'_> {
        TerminalView::show(&self.terminal)
            .map(|e| IdPaneMessage {
                id: self.id,
                msg: e.into(),
            })
            .map(AppMsg::from)
    }

    pub fn update(&mut self, msg: PaneMessage) -> AppTask {
        match msg {
            PaneMessage::TerminalMsg(iced_term::Event::BackendCall(_, command)) => {
                let action = self
                    .terminal
                    .handle(iced_term::Command::ProxyToBackend(command));

                if action == iced_term::actions::Action::Shutdown {
                    return AppTask::done(AppMsg::TabPaneClose { pane: self.id });
                }
            }
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
        tracing::debug!("Focus on pane {}", self.id);

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
    TerminalMsg(iced_term::Event),
}
