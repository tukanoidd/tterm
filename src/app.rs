pub mod components;

use std::{collections::HashMap, fmt::Display};

use derive_more::From;
use iced::{
    Length,
    alignment::Horizontal,
    keyboard::{self, Modifiers},
    mouse,
    widget::{self, center, column, rule, text, text_editor},
};
use iced_aw::Spinner;
use itertools::Itertools;
use strum::VariantArray;
use uuid::Uuid;

use crate::{
    CLI_PRESET,
    app::components::{keybind_bar::KeyBindBar, tab_bar::TabBar},
    config::{
        Config,
        keybinds::{
            KeyBind, KeyBindPanelType, Modifier, MoveFocusDirection, TTermAction,
            TTermGeneralAction, TTermPaneAction, TTermTabAction,
        },
        presets::PresetConfig,
    },
    multiplex::{
        pane::{IdPaneMessage, PaneMessage},
        tab::{Tab, TabPanesType},
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

                rename_tab_mode,
                rename_tab_content,

                config,

                panel_expanded,
                ..
            } => {
                let tab_widget = match tabs.get(*current_tab) {
                    None => center(Spinner::new().width(20).height(20)).into(),
                    Some(tab) => tab.view(),
                };

                column![
                    TabBar::new(tabs, *current_tab, *rename_tab_mode, rename_tab_content).view(),
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
                let current_preset_name = CLI_PRESET
                    .get()
                    .cloned()
                    .flatten()
                    .or_else(|| config.presets.default.clone());
                let current_preset = match current_preset_name {
                    Some(name) => match config.presets.list.iter().find(|p| p.name == name) {
                        Some(preset) => Some(preset),
                        None => {
                            tracing::debug!(
                                "Provided preset name through CLI or the 'default' field was not found! Using the first one, if available..."
                            );

                            config.presets.list.first()
                        }
                    },
                    None => {
                        tracing::debug!(
                            "Preset not specified through CLI or 'default' field in config! Using the first one, if available..."
                        );

                        config.presets.list.first()
                    }
                };

                let new_tab_tasks = match current_preset {
                    Some(PresetConfig { tabs, .. }) => tabs
                        .iter()
                        .map(|config| {
                            AppTask::done(TTermTabAction::New(Some(config.clone())).into())
                        })
                        .collect(),
                    None => vec![],
                };

                self.state = AppState::Main {
                    config,

                    tabs: vec![],
                    current_tab: 0,

                    rename_tab_mode: false,
                    rename_tab_content: text_editor::Content::new(),

                    panel_expanded: HashMap::new(),
                };

                return match new_tab_tasks.is_empty() {
                    true => AppTask::done(TTermTabAction::New(None).into()),
                    false => AppTask::batch(new_tab_tasks),
                };
            }

            AppMsg::RenameTabEditorAction(action) => {
                let rename_tab_content = get_main_state![rename_tab_content];

                match action {
                    text_editor::Action::Edit(text_editor::Edit::Enter) => {
                        return AppTask::done(AppMsg::RenameCurrentTab(rename_tab_content.text()));
                    }
                    action => rename_tab_content.perform(action),
                }
            }
            AppMsg::RenameCurrentTab(new_name) => {
                let new_name = new_name.trim();

                if new_name.is_empty() {
                    return AppTask::none();
                }

                let (tabs, current_tab, rename_tab_mode) =
                    get_main_state![tabs, current_tab, rename_tab_mode];

                let Some(tab) = tabs.get_mut(*current_tab) else {
                    return AppTask::none();
                };

                tab.name = Some(new_name.into());
                *rename_tab_mode = false;

                return AppTask::done(TTermTabAction::Select(*current_tab).into());
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
                    TTermTabAction::Select(tab.saturating_sub(1.clamp(0, tabs.len()))).into(),
                );
            }
            AppMsg::TabResetFloating(id) => {
                let tabs = get_main_state![tabs];
                let Some(tab) = tabs.iter_mut().find(|t| t.id == id) else {
                    return AppTask::none();
                };

                tab.panes.remove(&TabPanesType::Floating);

                return AppTask::done(TTermTabAction::FocusedToggleFloating.into());
            }
            AppMsg::FocusPane(id) => {
                let (tabs, current_tab) = get_main_state![tabs, current_tab];
                let Some(tab) = tabs.get_mut(*current_tab) else {
                    return AppTask::none();
                };

                return tab.focus_pane(id);
            }

            AppMsg::Pane(pane_msg) => {
                let tabs = get_main_state![tabs];

                return tabs
                    .iter_mut()
                    .find_map(|t| t.update_pane(&pane_msg))
                    .unwrap_or_else(AppTask::none);
            }

            AppMsg::PanelToggle { ty, force } => {
                let panel_expanded = get_main_state![panel_expanded];
                let entry = panel_expanded.entry(ty).or_default();

                *entry = force.unwrap_or(!*entry);
            }

            AppMsg::Action(tterm_action) => {
                let (tabs, current_tab, config, rename_tab_content) =
                    get_main_state![tabs, current_tab, config, rename_tab_content];

                match tterm_action {
                    TTermAction::Tab(act) => match act {
                        TTermTabAction::New(tab_config) => {
                            let (tab, task) = match Tab::builder()
                                .terminal_config(&config.terminal)
                                .keybinds_config(&config.keybinds)
                                .maybe_tab_config(tab_config)
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
                                AppTask::done(TTermTabAction::Select(tabs.len() - 1).into()),
                                task,
                            ]);
                        }
                        TTermTabAction::CloseFocused => {
                            return AppTask::done(AppMsg::CloseTab(tabs[*current_tab].id));
                        }
                        TTermTabAction::Select(index) => {
                            let index = index.clamp(0, tabs.len().saturating_sub(1));

                            *current_tab = index;

                            if !tabs.is_empty() {
                                let current_tab = &mut tabs[*current_tab];

                                *rename_tab_content = text_editor::Content::with_text(
                                    current_tab.name.as_deref().unwrap_or_default(),
                                );

                                let Some((_, pane_state)) = current_tab
                                    .panes
                                    .get_mut(&current_tab.current_panes_type)
                                    .and_then(|panes| panes.focused_pane_mut())
                                else {
                                    return AppTask::none();
                                };

                                return AppTask::done(AppMsg::FocusPane(pane_state.id));
                            }
                        }
                        TTermTabAction::FocusedToggleFloating => {
                            let Some(current_tab) = tabs.get_mut(*current_tab) else {
                                return AppTask::none();
                            };

                            return current_tab.toggle_floating(&config.terminal, &config.keybinds);
                        }
                        TTermTabAction::FocusedTogglePaneStacking => {
                            let Some(current_tab) = tabs.get_mut(*current_tab) else {
                                return AppTask::none();
                            };

                            current_tab.toggle_stacking();
                        }
                        TTermTabAction::ToggleRename => {
                            let rename_tab_mode = get_main_state!(rename_tab_mode);
                            *rename_tab_mode = true;

                            return widget::operation::focus("rename-tab-editor");
                        }
                    },
                    TTermAction::Pane(act) => match act {
                        TTermPaneAction::SplitFocused(direction) => {
                            let Some(current_tab) = tabs.get_mut(*current_tab) else {
                                return AppTask::none();
                            };

                            return match current_tab.split_focused(
                                direction,
                                &config.terminal,
                                &config.keybinds,
                            ) {
                                Ok(task) => task,
                                Err(err) => AppTask::done(AppMsg::Error {
                                    message: err.to_string(),
                                    critical: false,
                                }),
                            };
                        }
                        TTermPaneAction::CloseFocused => {
                            let Some(tab) = tabs.get_mut(*current_tab) else {
                                return AppTask::none();
                            };

                            tracing::debug!("Close focused pane");

                            return tab.close_focused_pane();
                        }
                        TTermPaneAction::MoveFocused(direction) => {
                            let Some(tab) = tabs.get_mut(*current_tab) else {
                                return AppTask::none();
                            };

                            return tab.move_focused_pane(direction);
                        }
                    },
                    TTermAction::General(act) => match act {
                        TTermGeneralAction::Focus(direction) => {
                            let Some(tab) = tabs.get_mut(*current_tab) else {
                                return AppTask::none();
                            };

                            match tab.focus_pane_directional(direction) {
                                Some(task) => return task,
                                None => match direction {
                                    MoveFocusDirection::Left => {
                                        if *current_tab != 0 {
                                            return AppTask::done(
                                                TTermTabAction::Select(*current_tab - 1).into(),
                                            );
                                        }
                                    }
                                    MoveFocusDirection::Right => {
                                        if *current_tab < tabs.len() - 1 {
                                            return AppTask::done(
                                                TTermTabAction::Select(*current_tab + 1).into(),
                                            );
                                        }
                                    }
                                    MoveFocusDirection::Up | MoveFocusDirection::Down => {}
                                },
                            }
                        }
                        TTermGeneralAction::KeyBindPanelsToggle => {
                            let panel_expanded = get_main_state![panel_expanded];
                            let expanded =
                                panel_expanded.values().next().copied().unwrap_or_default();

                            return AppTask::batch(KeyBindPanelType::VARIANTS.iter().map(|ty| {
                                AppTask::done(AppMsg::PanelToggle {
                                    ty: *ty,
                                    force: Some(!expanded),
                                })
                            }));
                        }
                    },
                }
            }

            AppMsg::IcedEvent(event) => match event {
                iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    repeat: false,
                    ..
                }) => {
                    let (rename_tab_mode, current_tab) =
                        get_main_state!(rename_tab_mode, current_tab);

                    if *rename_tab_mode {
                        *rename_tab_mode = false;
                        return AppTask::done(TTermTabAction::Select(*current_tab).into());
                    }
                }
                iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                    let (tabs, current_tab, rename_tab_mode, config) =
                        get_main_state![tabs, current_tab, rename_tab_mode, config];

                    if *rename_tab_mode {
                        return AppTask::none();
                    }

                    let Some(current_tab) = tabs.get(*current_tab) else {
                        return AppTask::none();
                    };

                    let Some((_, focused_pane)) = current_tab.focused_pane() else {
                        return AppTask::none();
                    };

                    let scroll_delta = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => {
                            (y * config.terminal.scroll_acceleration) as i32
                        }
                        mouse::ScrollDelta::Pixels { y, .. } => {
                            (y * config.terminal.scroll_acceleration / config.terminal.font.size)
                                as i32
                        }
                    };

                    if scroll_delta != 0 {
                        return AppTask::done(
                            IdPaneMessage {
                                id: focused_pane.id,
                                msg: PaneMessage::TerminalMsg(iced_term::Event::BackendCall(
                                    focused_pane.term_id,
                                    iced_term::BackendCommand::Scroll(scroll_delta),
                                )),
                            }
                            .into(),
                        );
                    }
                }
                _ => {
                    // TODO
                }
            },
        }

        AppTask::none()
    }

    pub fn subscription(&self) -> AppSubscription {
        let AppState::Main { tabs, config, .. } = &self.state else {
            return AppSubscription::none();
        };

        let keybind_subscription =
            iced::event::listen_with(move |event, _, _window_id| Some(event))
                .with((
                    config
                        .keybinds
                        .actions
                        .iter()
                        .map(|(keybind, action)| {
                            (
                                KeyBindPanelType::from(action),
                                keybind.clone(),
                                action.clone(),
                            )
                        })
                        .collect::<Vec<_>>(),
                    config.general.reactive_panels,
                ))
                .map(|((binds, reactive_panels), event)| match event {
                    iced::Event::Keyboard(keyboard_event) => match keyboard_event {
                        keyboard::Event::KeyPressed {
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
                                            _,
                                            KeyBind {
                                                key: bind_key,
                                                modifiers: bind_modifiers,
                                            },
                                            action,
                                        )| {
                                            let iced_key: iced::keyboard::Key = bind_key.into();
                                            let iced_modifiers = bind_modifiers.into_iter().fold(
                                                Modifiers::empty(),
                                                |mods, mod_| match mod_ {
                                                    Modifier::Ctrl => mods | Modifiers::CTRL,
                                                    Modifier::Shift => mods | Modifiers::SHIFT,
                                                    Modifier::Alt => mods | Modifiers::ALT,
                                                },
                                            );

                                            ([&key, &modified_key].contains(&&iced_key)
                                                && iced_modifiers == modifiers)
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
                        keyboard::Event::ModifiersChanged(modifiers) => match reactive_panels {
                            true => {
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
                                    .map(|(t, b, _)| (b, t))
                                    .unique()
                                    .map(|(b, ty)| {
                                        let open =
                                            b.modifiers.iter().any(|m| changed_mods.contains(m));

                                        AppMsg::PanelToggle {
                                            ty,
                                            force: Some(open),
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            }
                            false => {
                                vec![]
                            }
                        },
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

        rename_tab_mode: bool,
        rename_tab_content: text_editor::Content,

        panel_expanded: HashMap<KeyBindPanelType, bool>,
    },
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
    RenameTabEditorAction(text_editor::Action),
    #[from(skip)]
    RenameCurrentTab(String),
    #[from(skip)]
    CloseTab(Uuid),
    #[from(skip)]
    TabResetFloating(Uuid),
    #[from(skip)]
    FocusPane(Uuid),

    Pane(IdPaneMessage),

    #[from(skip)]
    PanelToggle {
        ty: KeyBindPanelType,
        force: Option<bool>,
    },

    #[from(skip)]
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
