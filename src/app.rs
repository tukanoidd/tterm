pub mod components;
pub mod mode;
pub mod state;

use std::fmt::Display;

use derive_more::From;
use iced::{
    alignment::Horizontal,
    keyboard, mouse,
    widget::{self, center, column, text},
};
use iced_aw::Spinner;
use iced_swdir_tree::DirectoryTreeEvent;
use itertools::Itertools;

use crate::{
    CLI_PRESET,
    app::{
        mode::{TTermMode, TTermModeVariant, TerminalMode, TerminalModeTabAction, WebViewMode},
        state::{directory_tree::DirectoryTreeState, tabs::TabsState, webview::WebViewState},
    },
    config::{
        Config,
        keybinds::{Modifier, MoveFocusDirection},
        presets::PresetConfig,
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
    config: Option<Box<Config>>,
    state: AppState,

    current_mode: TTermModeVariant,
}

impl App {
    pub fn boot() -> (Self, AppTask) {
        let res = Self {
            config: None,
            state: AppState::LoadingConfig,

            current_mode: TTermModeVariant::Terminal,
        };
        let task = AppTask::done(AppMsg::LoadConfig);

        (res, task)
    }

    pub fn title(&self) -> String {
        "TTerm".into()
    }

    pub fn theme(&self) -> AppTheme {
        match &self.config {
            Some(config) => config.general.theme.into(),
            None => AppTheme::Dark,
        }
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

            AppState::Main(state) => {
                let config = self.config.as_ref().unwrap();

                // let dir_tree_tab_widget_opt_webview: AppElement<'_> =
                //     match WebViewModal::new(webview_state).view() {
                //         Some(webview_modal_element) => {
                //             stack![dir_tree_tab_widget, webview_modal_element,].into()
                //         }
                //         None => dir_tree_tab_widget.into(),
                //     };

                match self.current_mode {
                    TTermModeVariant::Terminal => TerminalMode.view(config, state),
                    TTermModeVariant::WebView => todo!(),
                }
            }
        }
    }

    pub fn update(&mut self, msg: AppMsg) -> AppTask {
        macro_rules! get_main_state {
            ($($param:ident),+) => {{
                match &mut self.state {
                    AppState::Main(state) => {
                        let MainState {
                            $($param,)+ ..
                        } = state.as_mut();

                        ($($param),+)
                    },
                    _ => {
                        tracing::warn!("Not in main state...");
                        return AppTask::none();
                    }
                }
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
                            AppTask::done(TerminalModeTabAction::New(Some(config.clone())).into())
                        })
                        .collect(),
                    None => vec![],
                };

                let home_dir = match std::env::home_dir() {
                    Some(home_dir) => home_dir,
                    None => {
                        return AppTask::done(AppMsg::Error {
                            message: "Failed to find home directory".into(),
                            critical: true,
                        });
                    }
                };

                let (webview_state, webview_init_task) =
                    WebViewState::new(&config.webview.default_url);

                self.config = Some(config);
                self.state = AppState::Main(Box::new(MainState {
                    directory_tree_state: DirectoryTreeState::new(home_dir.clone()),
                    tabs_state: TabsState::default(),
                    webview_state,
                }));

                return AppTask::batch([
                    AppTask::done(AppMsg::TerminalMode(
                        DirectoryTreeEvent::Toggled(home_dir).into(),
                    )),
                    webview_init_task,
                    match new_tab_tasks.is_empty() {
                        true => AppTask::done(TerminalModeTabAction::New(None).into()),
                        false => AppTask::batch(new_tab_tasks),
                    },
                ]);
            }

            AppMsg::TerminalMode(msg) => {
                let Some((config, state)) = self.config.as_ref().and_then(|config| match &mut self
                    .state
                {
                    AppState::LoadingConfig => None,
                    AppState::Main(main_state) => Some((config, main_state)),
                }) else {
                    return AppTask::none();
                };

                return TerminalMode.update(msg, &config, state);
            }
            AppMsg::WebViewMode(msg) => {
                let Some((config, state)) = self.config.as_ref().and_then(|config| match &mut self
                    .state
                {
                    AppState::LoadingConfig => None,
                    AppState::Main(main_state) => Some((config, main_state)),
                }) else {
                    return AppTask::none();
                };

                return WebViewMode.update(msg, &config, state);
            }

            // AppMsg::Action(tterm_action) => {
            //     let tabs = get_main_state![tabs_state];
            //     let Some(config) = self.config.as_ref() else {
            //         return AppTask::none();
            //     };

            //     match tterm_action {
            //         TTermAction::Tab(act) => match act {
            //             TTermTabAction::New(tab_config) => {
            //                 let (tab, task) = match Tab::builder()
            //                     .terminal_config(&config.terminal)
            //                     .keybinds_config(&config.keybinds)
            //                     .maybe_tab_config(tab_config)
            //                     .build()
            //                 {
            //                     Ok(tab) => tab,
            //                     Err(err) => {
            //                         return AppTask::done(AppMsg::Error {
            //                             message: err.to_string(),
            //                             critical: true,
            //                         });
            //                     }
            //                 };

            //                 tabs.tabs.push(tab);

            //                 return AppTask::batch([
            //                     AppTask::done(TTermTabAction::Select(tabs.tabs.len() - 1).into()),
            //                     task,
            //                 ]);
            //             }
            //             TTermTabAction::CloseFocused => {
            //                 return AppTask::done(
            //                     <TerminalMode as TTermMode>::Message::CloseTab(
            //                         tabs.tabs[tabs.current].id,
            //                     )
            //                     .into(),
            //                 );
            //             }
            //             TTermTabAction::Select(index) => {
            //                 let index = index.clamp(0, tabs.tabs.len().saturating_sub(1));

            //                 tabs.current = index;

            //                 if tabs.tabs.is_empty() {
            //                     return AppTask::none();
            //                 }

            //                 let current_tab = &mut tabs.tabs[tabs.current];

            //                 tabs.rename_content = current_tab.name.clone().unwrap_or_default();

            //                 let Some((_, pane_state)) = current_tab
            //                     .panes
            //                     .get_mut(&current_tab.current_panes_type)
            //                     .and_then(|panes| panes.focused_pane_mut())
            //                 else {
            //                     return AppTask::none();
            //                 };

            //                 return AppTask::done(
            //                     <TerminalMode as TTermMode>::Message::FocusPane(pane_state.id)
            //                         .into(),
            //                 );
            //             }
            //             TTermTabAction::FocusedToggleFloating => {
            //                 let Some(current_tab) = tabs.current_tab_mut() else {
            //                     return AppTask::none();
            //                 };

            //                 return current_tab.toggle_floating(&config.terminal, &config.keybinds);
            //             }
            //             TTermTabAction::FocusedTogglePaneStacking => {
            //                 let Some(current_tab) = tabs.current_tab_mut() else {
            //                     return AppTask::none();
            //                 };

            //                 current_tab.toggle_stacking();
            //             }
            //             TTermTabAction::ToggleRename => {
            //                 let tabs = get_main_state!(tabs_state);
            //                 tabs.rename_mode = true;

            //                 return widget::operation::focus("rename-tab-editor");
            //             }
            //         },
            //         TTermAction::Pane(act) => match act {
            //             TTermPaneAction::SplitFocused(direction) => {
            //                 let Some(current_tab) = tabs.current_tab_mut() else {
            //                     return AppTask::none();
            //                 };

            //                 return match current_tab.split_focused(
            //                     direction,
            //                     &config.terminal,
            //                     &config.keybinds,
            //                 ) {
            //                     Ok(task) => task,
            //                     Err(err) => AppTask::done(AppMsg::Error {
            //                         message: err.to_string(),
            //                         critical: false,
            //                     }),
            //                 };
            //             }
            //             TTermPaneAction::CloseFocused => {
            //                 let Some(tab) = tabs.current_tab_mut() else {
            //                     return AppTask::none();
            //                 };

            //                 return tab.close_focused_pane();
            //             }
            //             TTermPaneAction::MoveFocused(direction) => {
            //                 let Some(tab) = tabs.current_tab_mut() else {
            //                     return AppTask::none();
            //                 };

            //                 return tab.move_focused_pane(direction);
            //             }
            //         },
            //         TTermAction::General(act) => match act {
            //             TTermGeneralAction::Focus(direction) => {
            //                 let Some(tab) = tabs.current_tab_mut() else {
            //                     return AppTask::none();
            //                 };

            //                 match tab.focus_pane_directional(direction) {
            //                     Some(task) => return task,
            //                     None => match direction {
            //                         MoveFocusDirection::Left => {
            //                             return AppTask::done(
            //                                 TTermTabAction::Select(match tabs.current {
            //                                     0 => tabs.tabs.len() - 1,
            //                                     _ => tabs.current - 1,
            //                                 })
            //                                 .into(),
            //                             );
            //                         }
            //                         MoveFocusDirection::Right => {
            //                             return AppTask::done(
            //                                 TTermTabAction::Select(
            //                                     match tabs.current >= tabs.tabs.len() - 1 {
            //                                         true => 0,
            //                                         false => tabs.current + 1,
            //                                     },
            //                                 )
            //                                 .into(),
            //                             );
            //                         }
            //                         _ => {}
            //                     },
            //                 }
            //             }
            //             TTermGeneralAction::KeyBindPanelsToggle => {
            //                 let panel_expanded = get_main_state![panel_expanded];
            //                 let expanded =
            //                     panel_expanded.values().next().copied().unwrap_or_default();

            //                 return AppTask::batch(KeyBindPanelType::VARIANTS.iter().map(|ty| {
            //                     AppTask::done(AppMsg::PanelToggle {
            //                         ty: *ty,
            //                         force: Some(!expanded),
            //                     })
            //                 }));
            //             }
            //             TTermGeneralAction::DirectoryTreeToggle => {
            //                 let (directory_tree, tabs) =
            //                     get_main_state!(directory_tree_state, tabs_state);
            //                 directory_tree.show = !directory_tree.show;

            //                 if let Some((_, pane)) =
            //                     tabs.current_tab().and_then(|t| t.focused_pane())
            //                 {
            //                     return AppTask::done(
            //                         <TerminalMode as TTermMode>::Message::FocusPane(pane.id).into(),
            //                     );
            //                 }
            //             }
            //             TTermGeneralAction::WebViewToggle => {
            //                 let (tabs, webview) = get_main_state![tabs_state, webview_state];

            //                 if let Some(after_toggle) = webview.toggle(
            //                     tabs.current_tab()
            //                         .and_then(|t| t.focused_pane())
            //                         .map(|x| x.1.id),
            //                 ) {
            //                     return after_toggle;
            //                 }
            //             }
            //         },
            //     }
            // }
            AppMsg::WebViewCreatedView => {
                let webview = get_main_state![webview_state];
                return webview.created_view();
            }
            AppMsg::WebView(action) => {
                let webview = get_main_state![webview_state];
                let Some(config) = self.config.as_ref() else {
                    return AppTask::none();
                };

                return webview.action(action, &config.webview);
            }
            AppMsg::UpdateUrlInput(new_url) => {
                let webview = get_main_state![webview_state];
                webview.update_url_input(new_url);
            }

            AppMsg::IcedEvent(event) => match event {
                iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    repeat: false,
                    ..
                }) => {
                    let tabs = get_main_state![tabs_state];

                    if tabs.rename_mode {
                        tabs.rename_mode = false;
                        return AppTask::done(TTermTabAction::Select(tabs.current).into());
                    }
                }
                iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::F5),
                    repeat: false,
                    ..
                }) => {
                    let webview = get_main_state![webview_state];
                    return webview.refresh();
                }
                iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                    let tabs = get_main_state![tabs_state];
                    let Some(config) = self.config.as_ref() else {
                        return AppTask::none();
                    };

                    if tabs.rename_mode {
                        return AppTask::none();
                    }

                    let Some(current_tab) = tabs.current_tab() else {
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
                            <TerminalMode as TTermMode>::Message::Pane(IdPaneMessage {
                                id: focused_pane.id,
                                msg: PaneMessage::TerminalMsg(iced_term::Event::BackendCall(
                                    focused_pane.term_id,
                                    iced_term::BackendCommand::Scroll(scroll_delta),
                                )),
                            })
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
        let AppState::Main(state) = &self.state else {
            return AppSubscription::none();
        };
        let Some(config) = &self.config else {
            return AppSubscription::none();
        };

        let MainState {
            tabs_state,
            webview_state,
            ..
        } = state.as_ref();

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
                    webview_state.show,
                ))
                .map(
                    |((binds, reactive_panels, show_webview), event)| match event {
                        iced::Event::Keyboard(keyboard_event) => match keyboard_event {
                            keyboard::Event::KeyPressed {
                                key,
                                modified_key,
                                physical_key,
                                location,
                                modifiers,
                                text,
                                repeat,
                            } => {
                                vec![
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
                                                    let iced_key: iced::keyboard::Key =
                                                        bind_key.into();
                                                    let iced_modifiers =
                                                        bind_modifiers.into_iter().fold(
                                                            keyboard::Modifiers::empty(),
                                                            |mods, mod_| match mod_ {
                                                                Modifier::Ctrl => {
                                                                    mods | keyboard::Modifiers::CTRL
                                                                }
                                                                Modifier::Shift => {
                                                                    mods
                                                                        | keyboard::Modifiers::SHIFT
                                                                }
                                                                Modifier::Alt => {
                                                                    mods | keyboard::Modifiers::ALT
                                                                }
                                                            },
                                                        );

                                                    ([&key, &modified_key].contains(&&iced_key)
                                                        && iced_modifiers == modifiers)
                                                        .then_some(AppMsg::Action(action))
                                                },
                                            )
                                        })
                                        .flatten()
                                        .and_then(|msg| match show_webview {
                                            true => match msg {
                                                AppMsg::Action(TTermAction::General(
                                                    TTermGeneralAction::KeyBindPanelsToggle
                                                    | TTermGeneralAction::WebViewToggle,
                                                )) => Some(msg),
                                                _ => None,
                                            },
                                            false => Some(msg),
                                        })
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
                                ]
                            }
                            keyboard::Event::ModifiersChanged(modifiers) => match reactive_panels {
                                true => {
                                    let changed_mods = modifiers
                                        .iter()
                                        .filter_map(|m| match m {
                                            keyboard::Modifiers::SHIFT => Some(Modifier::Shift),
                                            keyboard::Modifiers::CTRL => Some(Modifier::Ctrl),
                                            keyboard::Modifiers::ALT => Some(Modifier::Alt),
                                            _ => None,
                                        })
                                        .collect::<Vec<_>>();

                                    // binds
                                    //     .into_iter()
                                    //     .map(|(t, b, _)| (b, t))
                                    //     .unique()
                                    //     .map(|(b, ty)| {
                                    //         let open = b
                                    //             .modifiers
                                    //             .iter()
                                    //             .any(|m| changed_mods.contains(m));

                                    //         AppMsg::PanelToggle {
                                    //             ty,
                                    //             force: Some(open),
                                    //         }
                                    //     })
                                    //     .collect::<Vec<_>>()
                                    vec![]
                                }
                                false => {
                                    vec![]
                                }
                            },
                            ev => vec![AppMsg::IcedEvent(iced::Event::Keyboard(ev))],
                        },
                        _ => vec![AppMsg::IcedEvent(event)],
                    },
                );
        let tab_subscriptions =
            AppSubscription::batch(tabs_state.tabs.iter().map(Tab::subscription));
        let webview_subscription = webview_state.subscription();

        AppSubscription::batch([
            keybind_subscription.map(Into::into),
            tab_subscriptions,
            webview_subscription,
        ])
    }

    fn load_config() -> AppTask {
        AppTask::perform(async move { Config::new().await }, |res| {
            AppMsg::from_result(res.map(Box::new), Into::into, true)
        })
    }
}

pub enum AppState {
    LoadingConfig,
    Main(Box<MainState>),
}

pub struct MainState {
    pub directory_tree_state: DirectoryTreeState,
    pub tabs_state: TabsState,
    pub webview_state: WebViewState,
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

    TerminalMode(<TerminalMode as TTermMode<'static>>::Message),
    WebViewMode(<WebViewMode as TTermMode<'static>>::Message),

    // #[from(skip)]
    // PanelToggle {
    //     ty: KeyBindPanelType,
    //     force: Option<bool>,
    // },
    WebViewCreatedView,
    WebView(iced_webview::Action),
    UpdateUrlInput(String),

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
