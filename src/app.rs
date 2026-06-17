pub mod components;

use std::fmt::Display;

use derive_more::From;
use iced::{
    Length,
    alignment::Horizontal,
    keyboard::Modifiers,
    widget::{
        center, column,
        pane_grid::{self, ResizeEvent},
        rule, text,
    },
};
use iced_aw::Spinner;
use uuid::Uuid;

use crate::{
    app::components::{keybind_bar::KeyBindBar, tab_bar::TabBar},
    config::{
        Config,
        keybinds::{KeyBind, Modifier, TTermAction},
    },
    multiplex::{
        pane::{IdPaneMessage, PaneMessage},
        tab::Tab,
    },
};

pub type AppTheme = iced::Theme;
pub type AppRenderer = iced::Renderer;
pub type AppElement<'a, M = AppMsg> = iced::Element<'a, M, AppTheme, AppRenderer>;

pub type AppTask = iced::Task<AppMsg>;
pub type AppSubscription = iced::Subscription<AppMsg>;

pub struct App {
    theme: AppTheme,

    state: AppState,
}

impl App {
    pub fn boot() -> (Self, AppTask) {
        let res = Self {
            theme: AppTheme::TokyoNight,
            state: AppState::LoadingConfig,
        };
        let task = AppTask::done(AppMsg::LoadConfig);

        (res, task)
    }

    pub fn title(&self) -> String {
        "TTerm".into()
    }

    pub fn theme(&self) -> AppTheme {
        self.theme.clone()
    }

    pub fn view(&self) -> AppElement<'_> {
        match &self.state {
            AppState::LoadingConfig => center(
                column![
                    Spinner::new().width(20.0).height(20),
                    text("Loading config...")
                ]
                .align_x(Horizontal::Center),
            )
            .into(),

            AppState::Main {
                tabs,
                current_tab,

                config,

                tab_expanded,
                pane_expanded,
                general_expanded,
                ..
            } => {
                let tab_widget = match tabs.get(*current_tab) {
                    None => center(Spinner::new().width(20).height(20)).into(),
                    Some(tab) => tab.view(),
                };

                column![
                    TabBar::new(tabs, *current_tab).view(),
                    rule::horizontal(2),
                    tab_widget,
                    rule::horizontal(2),
                    KeyBindBar::new(
                        &config.keybinds,
                        *tab_expanded,
                        *pane_expanded,
                        *general_expanded
                    )
                    .view()
                ]
                .width(Length::Fill)
                .height(Length::Fill)
                .spacing(10)
                .padding(5)
                .into()
            }
        }
    }

    pub fn update(&mut self, msg: AppMsg) -> AppTask {
        match msg {
            AppMsg::Error { message, critical } => {
                tracing::error!("{message}");

                if critical {
                    return iced::exit();
                }
            }

            AppMsg::LoadConfig => return Self::load_config(),
            AppMsg::LoadedConfig(config) => {
                self.state = AppState::Main {
                    config,

                    tabs: Vec::new(),
                    current_tab: 0,

                    tab_expanded: false,
                    pane_expanded: false,
                    general_expanded: false,
                };

                return AppTask::done(TTermAction::NewTab.into());
            }

            AppMsg::CloseTab(id) => {
                let AppState::Main {
                    tabs, current_tab, ..
                } = &mut self.state
                else {
                    tracing::warn!("Not in main state...");
                    return AppTask::none();
                };

                let Some(tab) = tabs
                    .iter()
                    .enumerate()
                    .find_map(|(ind, tab)| (tab.id == id).then_some(ind))
                else {
                    return AppTask::none();
                };

                tabs.remove(tab);

                *current_tab = tab.saturating_sub(1);

                if tabs.is_empty() {
                    return iced::exit();
                }

                return AppTask::done(
                    TTermAction::SelectTab(tab.saturating_sub(1.clamp(0, tabs.len()))).into(),
                );
            }
            AppMsg::FocusPane(id) => {
                let AppState::Main {
                    tabs, current_tab, ..
                } = &mut self.state
                else {
                    tracing::warn!("Not in main state...");
                    return AppTask::none();
                };
                let Some(tab) = tabs.get_mut(*current_tab) else {
                    return AppTask::none();
                };

                tab.focused_pane = id;

                return tab
                    .pane(id)
                    .map(|p| p.focus())
                    .unwrap_or_else(AppTask::none);
            }

            AppMsg::Pane(IdPaneMessage { id, msg }) => {
                let AppState::Main { tabs, .. } = &mut self.state else {
                    tracing::warn!("Not in main state...");
                    return AppTask::none();
                };

                match msg {
                    PaneMessage::Resize(ResizeEvent { split, ratio }) => {
                        let Some(tab) = tabs.iter_mut().find(|tab| tab.id == id) else {
                            return AppTask::none();
                        };

                        tab.panes.resize(split, ratio);
                    }
                    PaneMessage::Dragged(event) => {
                        if let Some(tab) = tabs.iter_mut().find(|tab| tab.pane(id).is_some()) {
                            match event {
                                pane_grid::DragEvent::Picked { pane } => {
                                    let Some(p) = tab.panes.get(pane) else {
                                        return AppTask::none();
                                    };

                                    return AppTask::done(AppMsg::FocusPane(p.id));
                                }
                                pane_grid::DragEvent::Dropped { pane, target } => {
                                    tab.panes.drop(pane, target);

                                    let Some(p) = tab.panes.get(pane) else {
                                        return AppTask::none();
                                    };

                                    return AppTask::done(AppMsg::FocusPane(p.id));
                                }
                                pane_grid::DragEvent::Canceled { .. } => {}
                            }
                        }
                    }
                    PaneMessage::Close => {
                        let Some(tab) = tabs.iter_mut().find(|tab| tab.pane(id).is_some()) else {
                            return AppTask::none();
                        };

                        return tab.close_pane(id);
                    }
                    msg => {
                        let Some(pane) = tabs.iter_mut().find_map(|tab| tab.pane_mut(id)) else {
                            tracing::warn!("Failed to find pane {id}");
                            return AppTask::none();
                        };

                        return pane.update(msg);
                    }
                }
            }

            AppMsg::TabPanelToggle => {
                let AppState::Main { tab_expanded, .. } = &mut self.state else {
                    return AppTask::none();
                };

                *tab_expanded = !*tab_expanded;
            }
            AppMsg::PanePanelToggle => {
                let AppState::Main { pane_expanded, .. } = &mut self.state else {
                    return AppTask::none();
                };

                *pane_expanded = !*pane_expanded;
            }
            AppMsg::GeneralPanelToggle => {
                let AppState::Main {
                    general_expanded, ..
                } = &mut self.state
                else {
                    return AppTask::none();
                };

                *general_expanded = !*general_expanded;
            }

            AppMsg::Action(tterm_action) => {
                let AppState::Main {
                    tabs, current_tab, ..
                } = &self.state
                else {
                    return AppTask::none();
                };

                match tterm_action {
                    TTermAction::NewTab => {
                        let AppState::Main { config, tabs, .. } = &mut self.state else {
                            tracing::warn!("Not in main state...");
                            return AppTask::none();
                        };

                        let (tab, task) = match Tab::builder()
                            .terminal_config(config.terminal.clone())
                            .build()
                        {
                            Ok(tab) => tab,
                            Err(err) => {
                                return AppTask::done(AppMsg::Error {
                                    message: err.to_string(),
                                    critical: true,
                                });
                            }
                        };

                        tabs.push(tab);

                        return AppTask::batch([
                            AppTask::done(TTermAction::SelectTab(tabs.len() - 1).into()),
                            task,
                        ]);
                    }
                    TTermAction::CloseFocusedTab => {
                        return AppTask::done(AppMsg::CloseTab(tabs[*current_tab].id));
                    }
                    TTermAction::SelectTab(index) => {
                        let AppState::Main {
                            tabs, current_tab, ..
                        } = &mut self.state
                        else {
                            tracing::warn!("Not in main state...");
                            return AppTask::none();
                        };

                        let index = index.clamp(0, tabs.len().saturating_sub(1));

                        *current_tab = index;

                        if !tabs.is_empty()
                            && let Some(pane) =
                                tabs[*current_tab].pane(tabs[*current_tab].focused_pane)
                        {
                            return AppTask::done(AppMsg::FocusPane(pane.id));
                        }
                    }

                    TTermAction::SplitPaneVertical => {
                        // TODO
                    }
                    TTermAction::SplitPaneHorizontal => {
                        // TODO
                    }
                    TTermAction::CloseFocusedPane => {
                        let AppState::Main {
                            tabs, current_tab, ..
                        } = &self.state
                        else {
                            tracing::warn!("Not in main state or no focused pane...");
                            return AppTask::none();
                        };
                        let Some(tab) = tabs.get(*current_tab) else {
                            return AppTask::none();
                        };

                        return AppTask::done(
                            IdPaneMessage {
                                id: tab.focused_pane,
                                msg: PaneMessage::Close,
                            }
                            .into(),
                        );
                    }

                    TTermAction::FocusLeft => {
                        // TODO
                    }
                    TTermAction::FocusRight => {
                        // TODO
                    }
                    TTermAction::FocusUp => {
                        // TODO
                    }
                    TTermAction::FocusDown => {
                        // TODO
                    }
                }
            }

            AppMsg::IcedEvent(event) => {
                if let iced::Event::Keyboard(ke) = event {
                    tracing::trace!("...... {ke:?}");
                }
                // TODO
            }
        }

        AppTask::none()
    }

    pub fn subscription(&self) -> AppSubscription {
        let AppState::Main { tabs, config, .. } = &self.state else {
            return AppSubscription::none();
        };

        let keybind_subscription = iced::event::listen()
            .with(
                config
                    .keybinds
                    .actions
                    .iter()
                    .map(|(k, a)| (k.clone(), a.clone()))
                    .collect::<Vec<_>>(),
            )
            .map(|(binds, event)| match event {
                iced::Event::Keyboard(keyboard_event) => match keyboard_event {
                    iced::keyboard::Event::KeyReleased {
                        key,
                        modified_key,
                        physical_key,
                        location,
                        modifiers,
                    } => {
                        tracing::trace!("{key:?}, {modifiers:?}");

                        binds
                            .into_iter()
                            .find_map(
                                |(
                                    KeyBind {
                                        key: bind_key,
                                        modifiers: bind_modifiers,
                                    },
                                    action,
                                )| {
                                    let iced_key: iced::keyboard::Key = bind_key.into();
                                    let iced_modifiers = bind_modifiers
                                        .map(|bmods| {
                                            bmods.into_iter().fold(
                                                Modifiers::empty(),
                                                |mods, mod_| match mod_ {
                                                    Modifier::Ctrl => mods | Modifiers::CTRL,
                                                    Modifier::Shift => mods | Modifiers::SHIFT,
                                                    Modifier::Alt => mods | Modifiers::ALT,
                                                },
                                            )
                                        })
                                        .unwrap_or(Modifiers::empty());

                                    (iced_key == key && iced_modifiers == modifiers)
                                        .then_some(AppMsg::Action(action))
                                },
                            )
                            .unwrap_or_else(|| {
                                tracing::trace!("Skipped {key:?}, {modifiers:?}");

                                AppMsg::IcedEvent(iced::Event::Keyboard(
                                    iced::keyboard::Event::KeyReleased {
                                        key,
                                        modified_key,
                                        physical_key,
                                        location,
                                        modifiers,
                                    },
                                ))
                            })
                    }
                    // iced::keyboard::Event::ModifiersChanged(modifiers) => {
                    //     // TODO: open relevant popups (configurable)
                    // }
                    ev => AppMsg::IcedEvent(iced::Event::Keyboard(ev)),
                },
                _ => AppMsg::IcedEvent(event),
            });
        let tab_subscriptions = tabs.iter().map(Tab::subscription);

        AppSubscription::batch([keybind_subscription].into_iter().chain(tab_subscriptions))
    }

    fn load_config() -> AppTask {
        AppTask::perform(async move { Config::new().await.map(Box::new) }, |res| {
            AppMsg::from_result(res, Into::into, true)
        })
    }
}

pub enum AppState {
    LoadingConfig,
    Main {
        config: Box<Config>,

        tabs: Vec<Tab>,
        current_tab: usize,

        tab_expanded: bool,
        pane_expanded: bool,
        general_expanded: bool,
    },
}

#[derive(Debug, Clone, From)]
pub enum AppMsg {
    #[from(skip)]
    Error {
        message: String,
        critical: bool,
    },

    LoadConfig,
    LoadedConfig(Box<Config>),

    #[from(skip)]
    CloseTab(Uuid),
    #[from(skip)]
    FocusPane(Uuid),

    Pane(IdPaneMessage),

    TabPanelToggle,
    PanePanelToggle,
    GeneralPanelToggle,

    Action(TTermAction),

    IcedEvent(iced::Event),
}

impl AppMsg {
    fn from_result<T, E: Display>(
        result: std::result::Result<T, E>,
        on_ok: impl FnOnce(T) -> AppMsg,
        error_critical: bool,
    ) -> Self {
        match result {
            Ok(val) => on_ok(val),
            Err(err) => AppMsg::Error {
                message: err.to_string(),
                critical: error_critical,
            },
        }
    }
}
