pub mod components;

use iced::keyboard;

use tterm_macros::mode;

use crate::{
    app::{
        AppElement, AppMsg, AppSubscription, AppTask, mode::TTermModeVariant,
        state::webview::WebViewState,
    },
    config::webview::WebViewConfig,
};

use components::modal::WebViewModal;

mode! {
    WebView: {
        message: {
            WebViewCreatedView,
            WebView(iced_webview::Action),
            UpdateUrlInput(String),

            IcedEvent(iced::Event),
        };
        actions: {
            General: [
                Refresh    [ @[Ctrl] + "R" ],
                ToTerminal [ @[Alt+Shift] + "T" ],
            ]
        };

        config: {
            base: WebViewConfig
        };

        state: {
            webview: WebViewState,
        };

        fn view_impl<'a>(
            self,
            _config: &'a Self::Config,
            Self::State {
                webview,
                ..
            }: &'a Self::State
        ) -> impl Into<AppElement<'a>> {
            WebViewModal::new(webview).view()
        }

        fn update_impl<'a>(
            self,
            message: Self::Message,
            config: &'a Self::Config,
            state: &'a mut Self::State
        ) -> AppTask {
            match message {
                Self::Message::Action(action) => match action {
                    WebViewModeAction::General(general_action) => match general_action {
                        WebViewModeGeneralAction::Refresh => {
                            return state.webview.refresh();
                        },
                        WebViewModeGeneralAction::ToTerminal => {
                            return AppTask::done(AppMsg::SwitchMode(TTermModeVariant::Terminal));
                        }
                    },
                },
                Self::Message::PanelToggle {ty, force} => {
                    state.panel_toggle(ty, force);
                }

                Self::Message::WebViewCreatedView => {
                    return state.webview.created_view();
                }
                Self::Message::WebView(action) => {
                    return state.webview.action(action, &config.base);
                }
                Self::Message::UpdateUrlInput(new_url) => {
                    state.webview.update_url_input(new_url);
                }

                Self::Message::IcedEvent(event) => match event {
                    iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(keyboard::key::Named::F5),
                        repeat: false,
                        ..
                    }) => return AppTask::done(WebViewModeGeneralAction::Refresh.into()),
                    _ => {
                        // TODO
                    }
                }
            }

            AppTask::none()
        }

        fn subscription_impl<'a>(
            self,
            _config: &'a Self::Config,
            state: &'a Self::State
        ) -> AppSubscription {
            state.webview.subscription()
        }
    }
}
