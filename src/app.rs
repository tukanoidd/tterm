pub mod components;
pub mod mode;
pub mod state;

use std::{fmt::Display, sync::Arc};

use derive_more::From;
use directories::ProjectDirs;
use iced::{
    Event,
    alignment::Horizontal,
    futures::lock::Mutex,
    widget::{center, column, rule, text},
};
use iced_aw::Spinner;
use iced_swdir_tree::DirectoryTreeEvent;

use crate::{
    CLI_PRESET,
    app::{
        components::keybind_bar::KeyBindBar,
        mode::{
            TTermMode, TTermModeVariant,
            terminal::{
                TerminalMode, TerminalModeMessage, TerminalModeState, TerminalModeTabAction,
            },
            webview::{WebViewMode, WebViewModeMessage, WebViewModeState},
        },
        state::{
            db::DbState, directory_tree::DirectoryTreeState, tabs::TabsState, webview::WebViewState,
        },
    },
    config::{Config, presets::PresetConfig},
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
        let project_dirs = ProjectDirs::from("com", "tukanoid", "tterm").unwrap();

        let res = Self {
            config: None,
            state: AppState::LoadingConfig { project_dirs },

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
            AppState::LoadingConfig { .. } => center(
                column![
                    Spinner::new().width(20.0).height(20),
                    text("Loading config...")
                ]
                .align_x(Horizontal::Center),
            )
            .into(),

            AppState::Main(state) => {
                let config = self.config.as_ref().unwrap();

                column![
                    match self.current_mode {
                        TTermModeVariant::Terminal => TerminalMode.view(config, state),
                        TTermModeVariant::WebView => WebViewMode.view(config, state),
                    },
                    rule::horizontal(2),
                    match self.current_mode {
                        TTermModeVariant::Terminal => KeyBindBar::<TerminalMode>::new(
                            &config.terminal_mode.keybinds,
                            &state.terminal_mode.keybind_panel_expanded
                        )
                        .view(),
                        TTermModeVariant::WebView => KeyBindBar::<WebViewMode>::new(
                            &config.webview_mode.keybinds,
                            &state.webview_mode.keybind_panel_expanded
                        )
                        .view(),
                    },
                ]
                .spacing(10)
                .padding(10)
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

            AppMsg::Multiple(list) => return AppTask::batch(list.into_iter().map(AppTask::done)),

            AppMsg::LoadConfig => {
                let project_dirs = match &self.state {
                    AppState::LoadingConfig { project_dirs } => project_dirs,
                    AppState::Main(main_state) => &main_state.project_dirs,
                }
                .clone();

                return Self::load_config(project_dirs);
            }
            AppMsg::LoadedConfig(config) => {
                let project_dirs = match &self.state {
                    AppState::LoadingConfig { project_dirs } => project_dirs,
                    AppState::Main(main_state) => &main_state.project_dirs,
                }
                .clone();

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
                    WebViewState::new(&config.webview_mode.base.default_url);

                self.config = Some(config);
                self.state = AppState::Main(Box::new(MainState {
                    project_dirs,

                    terminal_mode: TerminalModeState {
                        keybind_panel_expanded: Default::default(),

                        directory_tree: DirectoryTreeState::new(home_dir.clone()),
                        tabs: TabsState::default(),
                    },
                    webview_mode: WebViewModeState {
                        keybind_panel_expanded: Default::default(),

                        webview: webview_state,
                    },

                    db: None,
                }));

                return AppTask::batch([
                    AppTask::done(AppMsg::LoadDb),
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

            AppMsg::LoadDb => {
                let project_dirs = match &self.state {
                    AppState::LoadingConfig { project_dirs } => project_dirs,
                    AppState::Main(main_state) => &main_state.project_dirs,
                }
                .clone();

                return AppTask::perform(DbState::new(project_dirs), |res| {
                    AppMsg::from_result(res.map(Mutex::new).map(Arc::new), AppMsg::LoadedDb, true)
                });
            }
            AppMsg::LoadedDb(db) => match &mut self.state {
                AppState::Main(main_state) => main_state.db = Some(db),
                _ => {}
            },

            AppMsg::SwitchMode(mode) => {
                self.current_mode = mode;
            }

            AppMsg::TerminalMode(msg) => {
                let Some((config, state)) = self.config.as_ref().and_then(|config| match &mut self
                    .state
                {
                    AppState::LoadingConfig { .. } => None,
                    AppState::Main(main_state) => Some((config, main_state)),
                }) else {
                    return AppTask::none();
                };

                return TerminalMode.update(msg, config, state);
            }
            AppMsg::WebViewMode(msg) => {
                let Some((config, state)) = self.config.as_ref().and_then(|config| match &mut self
                    .state
                {
                    AppState::LoadingConfig { .. } => None,
                    AppState::Main(main_state) => Some((config, main_state)),
                }) else {
                    return AppTask::none();
                };

                return WebViewMode.update(msg, config, state);
            }

            AppMsg::IcedEvent(event) => {
                return AppTask::done(match self.current_mode {
                    TTermModeVariant::Terminal => TerminalModeMessage::IcedEvent(event).into(),
                    TTermModeVariant::WebView => WebViewModeMessage::IcedEvent(event).into(),
                });
            }
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

        AppSubscription::batch([
            TerminalMode.subscription(config, state),
            WebViewMode.subscription(config, state),
        ])
    }

    fn load_config(project_dirs: ProjectDirs) -> AppTask {
        AppTask::perform(async move { Config::new(project_dirs).await }, |res| {
            AppMsg::from_result(res.map(Box::new), Into::into, true)
        })
    }
}

pub enum AppState {
    LoadingConfig { project_dirs: ProjectDirs },
    Main(Box<MainState>),
}

#[derive(derive_more::AsRef, derive_more::AsMut)]
pub struct MainState {
    pub project_dirs: ProjectDirs,

    pub terminal_mode: TerminalModeState,
    pub webview_mode: WebViewModeState,

    pub db: Option<Arc<Mutex<DbState>>>,
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

    LoadDb,
    LoadedDb(Arc<Mutex<DbState>>),

    SwitchMode(TTermModeVariant),
    TerminalMode(<TerminalMode as TTermMode>::Message),
    WebViewMode(<WebViewMode as TTermMode>::Message),

    IcedEvent(Event),
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
