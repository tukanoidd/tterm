pub mod components;

use std::{collections::HashMap, fmt::Display};

use derive_more::{Display, From};
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
use itertools::Itertools;
use uuid::Uuid;

use crate::{
    app::components::{keybind_bar::KeyBindBar, tab_bar::TabBar},
    config::{
        Config,
        keybinds::{FocusDirection, KeyBind, Modifier, TTermAction},
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

                panel_expanded,
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
                    KeyBindBar::new(&config.keybinds, panel_expanded).view()
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
        macro_rules! get_main_state {
            ($($param:ident),+) => {{
                let AppState::Main {
                    $($param,)+ ..
                } = &mut self.state
                else {
                    tracing::warn!("Not in main state...");
                    return AppTask::none();
                };

                ($($param),+)
            }};
        }

        match msg {
            AppMsg::Error { message, critical } => {
                tracing::error!("{message}");

                if critical {
                    return iced::exit();
                }
            }

            AppMsg::Multiple(list) => return AppTask::batch(list.into_iter().map(AppTask::done)),

            AppMsg::LoadConfig => return Self::load_config(),
            AppMsg::LoadedConfig(config) => {
                self.state = AppState::Main {
                    config,

                    tabs: Vec::new(),
                    current_tab: 0,

                    panel_expanded: HashMap::new(),
                };

                return AppTask::done(TTermAction::NewTab.into());
            }

            AppMsg::CloseTab(id) => {
                let (tabs, current_tab) = get_main_state![tabs, current_tab];

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
                let (tabs, current_tab) = get_main_state![tabs, current_tab];
                let Some(tab) = tabs.get_mut(*current_tab) else {
                    return AppTask::none();
                };

                tab.focused_pane = id;

                return tab
                    .pane(id)
                    .map(|(_, p)| p.focus())
                    .unwrap_or_else(AppTask::none);
            }

            AppMsg::Pane(IdPaneMessage { id, msg }) => {
                let tabs = get_main_state![tabs];

                match msg {
                    PaneMessage::Resize(ResizeEvent { split, ratio }) => {
                        let Some(tab) = tabs.iter_mut().find(|tab| tab.pane(id).is_some()) else {
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
                        let Some((_, pane_state)) =
                            tabs.iter_mut().find_map(|tab| tab.pane_mut(id))
                        else {
                            tracing::warn!("Failed to find pane {id}");
                            return AppTask::none();
                        };

                        return pane_state.update(msg);
                    }
                }
            }

            AppMsg::PanelToggle { ty, force } => {
                let panel_expanded = get_main_state![panel_expanded];
                let entry = panel_expanded.entry(ty).or_default();

                *entry = force.unwrap_or(!*entry);
            }

            AppMsg::Action(tterm_action) => {
                let (tabs, current_tab, config) = get_main_state![tabs, current_tab, config];

                match tterm_action {
                    TTermAction::NewTab => {
                        let (tab, task) =
                            match Tab::builder().terminal_config(&config.terminal).build() {
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
                        let index = index.clamp(0, tabs.len().saturating_sub(1));

                        *current_tab = index;

                        if !tabs.is_empty()
                            && let Some((_, pane_state)) =
                                tabs[*current_tab].pane(tabs[*current_tab].focused_pane)
                        {
                            return AppTask::done(AppMsg::FocusPane(pane_state.id));
                        }
                    }

                    TTermAction::SplitFocusedPane(direction) => {
                        let Some(current_tab) = tabs.get_mut(*current_tab) else {
                            return AppTask::none();
                        };

                        return match current_tab.split_focused(direction, &config.terminal) {
                            Ok(task) => task,
                            Err(err) => AppTask::done(AppMsg::Error {
                                message: err.to_string(),
                                critical: false,
                            }),
                        };
                    }
                    TTermAction::CloseFocusedPane => {
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

                    TTermAction::Focus(direction) => {
                        let Some(tab) = tabs.get_mut(*current_tab) else {
                            return AppTask::none();
                        };

                        match tab.focus_pane(direction) {
                            Some(task) => return task,
                            None => match direction {
                                FocusDirection::Left => {
                                    if *current_tab != 0 {
                                        return AppTask::done(
                                            TTermAction::SelectTab(*current_tab - 1).into(),
                                        );
                                    }
                                }
                                FocusDirection::Right => {
                                    if *current_tab < tabs.len() - 1 {
                                        return AppTask::done(
                                            TTermAction::SelectTab(*current_tab + 1).into(),
                                        );
                                    }
                                }
                                FocusDirection::Up | FocusDirection::Down => {}
                            },
                        }
                    }
                }
            }

            AppMsg::IcedEvent(_event) => {
                // TODO
            }
        }

        AppTask::none()
    }

    pub fn subscription(&self) -> AppSubscription {
        let AppState::Main { tabs, config, .. } = &self.state else {
            return AppSubscription::none();
        };

        let keybind_subscription =
            iced::event::listen_with(move |event, _, _window_id| Some(event))
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
                        iced::keyboard::Event::KeyPressed {
                            key,
                            modified_key,
                            physical_key,
                            location,
                            modifiers,
                            text,
                            repeat,
                        } => vec![
                            (!repeat)
                                .then(|| {
                                    binds.into_iter().find_map(
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
                                                            Modifier::Ctrl => {
                                                                mods | Modifiers::CTRL
                                                            }
                                                            Modifier::Shift => {
                                                                mods | Modifiers::SHIFT
                                                            }
                                                            Modifier::Alt => mods | Modifiers::ALT,
                                                        },
                                                    )
                                                })
                                                .unwrap_or(Modifiers::empty());

                                            (iced_key == key && iced_modifiers == modifiers)
                                                .then_some(AppMsg::Action(action))
                                        },
                                    )
                                })
                                .flatten()
                                .unwrap_or_else(|| {
                                    AppMsg::IcedEvent(iced::Event::Keyboard(
                                        iced::keyboard::Event::KeyPressed {
                                            key,
                                            modified_key,
                                            physical_key,
                                            location,
                                            modifiers,
                                            text,
                                            repeat,
                                        },
                                    ))
                                }),
                        ],
                        iced::keyboard::Event::ModifiersChanged(modifiers) => {
                            let changed_mods = modifiers
                                .iter()
                                .filter_map(|m| match m {
                                    Modifiers::SHIFT => Some(Modifier::Shift),
                                    Modifiers::CTRL => Some(Modifier::Ctrl),
                                    Modifiers::ALT => Some(Modifier::Alt),
                                    _ => None,
                                })
                                .collect::<Vec<_>>();

                            binds
                                .into_iter()
                                .map(|(b, a)| (b, KeyBindPanelType::from(a)))
                                .unique()
                                .map(|(b, ty)| {
                                    let open = b
                                        .modifiers
                                        .as_ref()
                                        .map(|mods| mods.iter().any(|m| changed_mods.contains(m)))
                                        .unwrap_or_default();

                                    AppMsg::PanelToggle {
                                        ty,
                                        force: Some(open),
                                    }
                                })
                                .collect::<Vec<_>>()
                        }
                        ev => vec![AppMsg::IcedEvent(iced::Event::Keyboard(ev))],
                    },
                    _ => vec![AppMsg::IcedEvent(event)],
                });
        let tab_subscriptions = tabs
            .iter()
            .map(Tab::subscription)
            .map(|s| s.map(|m| vec![m]));

        AppSubscription::batch(
            [keybind_subscription]
                .into_iter()
                .chain(tab_subscriptions)
                .map(|s| s.map(Into::into)),
        )
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

        panel_expanded: HashMap<KeyBindPanelType, bool>,
    },
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyBindPanelType {
    Tab,
    Pane,
    General,
}

impl KeyBindPanelType {
    pub fn title(&self) -> String {
        format!("{self} Panel")
    }
}

impl From<TTermAction> for KeyBindPanelType {
    fn from(value: TTermAction) -> Self {
        match value {
            TTermAction::NewTab | TTermAction::CloseFocusedTab | TTermAction::SelectTab(_) => {
                Self::Tab
            }
            TTermAction::SplitFocusedPane(_) | TTermAction::CloseFocusedPane => Self::Pane,
            TTermAction::Focus(_) => Self::General,
        }
    }
}

#[derive(Debug, Clone, From)]
pub enum AppMsg {
    #[from(skip)]
    Error {
        message: String,
        critical: bool,
    },

    Multiple(Vec<AppMsg>),

    LoadConfig,
    LoadedConfig(Box<Config>),

    #[from(skip)]
    CloseTab(Uuid),
    #[from(skip)]
    FocusPane(Uuid),

    Pane(IdPaneMessage),

    #[from(skip)]
    PanelToggle {
        ty: KeyBindPanelType,
        force: Option<bool>,
    },

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
