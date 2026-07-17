use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use bon::bon;
use derive_more::{Debug, From};
use iced::{
    Length,
    keyboard::Modifiers,
    widget::{button, column, container, mouse_area, pane_grid, text},
};
use iced_aw::ContextMenu;
use iced_term::{TermMode, Terminal, TerminalView};
use rootcause::Result;
use uuid::Uuid;

static TERM_ID: AtomicU64 = AtomicU64::new(0);

use crate::{
    app::{
        AppElement, AppMsg, AppSubscription, AppTask,
        mode::{
            TTermMode,
            terminal::{TerminalMode, TerminalModePaneAction},
        },
    },
    config::{
        keybinds::{Key, KeyBind, KeyBindsConfig, Modifier},
        presets::ProgramConfig,
        terminal::TerminalConfig,
    },
};

#[derive(Debug)]
pub struct PaneState {
    pub id: Uuid,
    pub term_id: u64,
    pub pwd: PathBuf,

    #[debug(skip)]
    terminal: Terminal,
}

#[bon]
impl PaneState {
    #[builder]
    pub fn new(
        id: Uuid,
        terminal_config: &TerminalConfig,
        keybinds_config: &KeyBindsConfig<TerminalMode>,
        working_directory: Option<PathBuf>,
        program_config: Option<ProgramConfig>,
    ) -> Result<Self> {
        let term_id = TERM_ID.load(Ordering::SeqCst);

        TERM_ID.store(term_id + 1, Ordering::SeqCst);

        let TerminalConfig {
            font, theme, shell, ..
        } = terminal_config;

        let env = std::env::vars().collect::<HashMap<_, _>>();

        let (program, args) = match program_config {
            Some(ProgramConfig { command, args }) => (Some(command), Some(args)),
            None => (None, None),
        };

        let working_directory = match working_directory {
            Some(d) => PathBuf::from(shellexpand::full(&d.to_string_lossy())?.to_string()),
            None => std::env::current_dir()?,
        };
        let mut terminal = Terminal::new(
            term_id,
            iced_term::settings::Settings {
                font: font.clone().into(),
                theme: theme.clone().into(),
                backend: iced_term::settings::BackendSettings {
                    program: program
                        .map(|p| shellexpand::full(&p).map(|p| p.to_string()))
                        .or_else(|| {
                            shell
                                .as_ref()
                                .map(|shell| shellexpand::full(&shell).map(|s| s.to_string()))
                        })
                        .transpose()?
                        .or_else(|| std::env::var("SHELL").ok())
                        .unwrap_or("nu".into()), // This is my preferred shell, deal with it
                    args: args.unwrap_or_default(),
                    env,
                    working_directory: Some(working_directory.clone()),
                },
            },
        )?;
        terminal.handle(iced_term::Command::AddBindings(
            keybinds_config
                .actions
                .iter()
                .flat_map(|(_, l)| l)
                .map(|(KeyBind { key, modifiers }, _)| {
                    (
                        iced_term::bindings::Binding {
                            target: match key {
                                Key::Named(named_key) => {
                                    iced_term::bindings::InputKind::KeyCode((*named_key).into())
                                }
                                Key::Character(char) => {
                                    iced_term::bindings::InputKind::Char(char.clone())
                                }
                            },
                            modifiers: modifiers.iter().fold(Modifiers::empty(), |ms, m| {
                                ms | match m {
                                    Modifier::Ctrl => Modifiers::CTRL,
                                    Modifier::Shift => Modifiers::SHIFT,
                                    Modifier::Alt => Modifiers::ALT,
                                }
                            }),
                            terminal_mode_include: TermMode::empty(),
                            terminal_mode_exclude: TermMode::all(),
                        },
                        iced_term::bindings::BindingAction::Ignore,
                    )
                })
                .collect(),
        ));

        Ok(Self {
            id,
            term_id,
            pwd: working_directory,
            terminal,
        })
    }

    pub fn view(&self, is_focused: bool) -> AppElement<'_> {
        let pane_view = container(
            mouse_area(
                TerminalView::show(&self.terminal)
                    .map(|e| IdPaneMessage {
                        id: self.id,
                        msg: e.into(),
                    })
                    .map(<TerminalMode as TTermMode>::Message::from)
                    .map(AppMsg::from),
            )
            .on_enter(<TerminalMode as TTermMode>::Message::FocusPane(self.id).into()),
        )
        .padding(4)
        .style(move |theme| {
            let palette = theme.extended_palette();

            let style = container::bordered_box(theme);
            style.border(style.border.color(match is_focused {
                true => palette.primary.strong.color,
                false => palette.secondary.base.color,
            }))
        });
        let selection_items = || {
            column(
                TerminalModePaneAction::default_keybinds()
                    .into_iter()
                    .map(|(_, action)| {
                        button(text(action.to_string()))
                            .on_press(action.into())
                            .style(button::subtle)
                            .width(Length::Fixed(300.0))
                            .into()
                    }),
            )
            .into()
        };

        ContextMenu::new(pane_view, selection_items).into()
    }

    pub fn update(&mut self, msg: &PaneMessage, is_focused: bool) -> Option<AppTask> {
        match msg {
            PaneMessage::TerminalMsg(iced_term::Event::BackendCall(_, command)) => {
                let action = self
                    .terminal
                    .handle(iced_term::Command::ProxyToBackend(command.clone()));

                match action {
                    iced_term::actions::Action::Shutdown => {
                        return Some(AppTask::done(
                            <TerminalMode as TTermMode>::Message::from(IdPaneMessage {
                                id: self.id,
                                msg: PaneMessage::Close,
                            })
                            .into(),
                        ));
                    }
                    iced_term::actions::Action::ChangeTitle(new_title) => {
                        let maybe_path = PathBuf::from(new_title.clone());

                        let mut pwd_switched = false;

                        match maybe_path.exists() {
                            true => {
                                if maybe_path.is_dir() {
                                    self.pwd = maybe_path;
                                    pwd_switched = true;
                                }
                            }
                            false => match shellexpand::full(&new_title) {
                                Ok(expanded_path_str) => {
                                    let expanded_path =
                                        PathBuf::from(expanded_path_str.to_string());

                                    if expanded_path.exists() && expanded_path.is_dir() {
                                        pwd_switched = true;
                                        self.pwd = expanded_path;
                                    }
                                }
                                Err(_) => {
                                    // TODO: maybe check for errors related to path being faulty or smth
                                }
                            },
                        }

                        if pwd_switched && is_focused {
                            return Some(AppTask::done(
                                <TerminalMode as TTermMode>::Message::UpdateFocusedDirectoryTree
                                    .into(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
            _ => unreachable!(),
        }

        None
    }

    pub fn subscription(&self) -> AppSubscription {
        let id = self.id;

        self.terminal
            .subscription()
            .with(id)
            .map(|(id, e)| IdPaneMessage { id, msg: e.into() })
            .map(<TerminalMode as TTermMode>::Message::from)
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
