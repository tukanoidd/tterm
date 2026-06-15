use std::fmt::Display;

use derive_more::From;
use iced::{
    alignment::Horizontal,
    widget::{center, column, text},
};
use iced_aw::Spinner;

use crate::{
    config::Config,
    multiplex::{pane::IdPaneMessage, tab::Tab},
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
                tabs, current_tab, ..
            } => {
                match tabs.get(*current_tab) {
                    None => center(Spinner::new().width(20).height(20)).into(),
                    Some(tab) => {
                        // TODO: selectable list of tabs

                        tab.view()
                    }
                }
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
                };

                return AppTask::done(AppMsg::InitTab);
            }
            AppMsg::InitTab => {
                let AppState::Main {
                    config,

                    tabs,
                    current_tab,
                } = &mut self.state
                else {
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
                *current_tab = tabs.len() - 1;

                return task;
            }
            AppMsg::Pane(IdPaneMessage { id, msg }) => {
                let AppState::Main { tabs, .. } = &mut self.state else {
                    return AppTask::none();
                };

                let Some(pane) = tabs.iter_mut().find_map(|tab| tab.pane_mut(id)) else {
                    return AppTask::none();
                };

                return pane.update(msg);
            }
        }

        AppTask::none()
    }

    pub fn subscription(&self) -> AppSubscription {
        let AppState::Main { tabs, .. } = &self.state else {
            return AppSubscription::none();
        };

        AppSubscription::batch(tabs.iter().map(Tab::subscription))
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

    InitTab,

    Pane(IdPaneMessage),
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
