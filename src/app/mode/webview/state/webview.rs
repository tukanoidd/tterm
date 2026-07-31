use iced::mouse;
use iced_webview::{PageType, WebView};

use crate::{
    app::{AppMsg, AppSubscription, AppTask, mode::webview::WebViewModeMessage},
    config::webview::WebViewConfig,
};

pub type WebViewEngine = iced_webview::Servo;
pub type AppWebView = WebView<WebViewEngine, AppMsg>;

pub struct WebViewState {
    pub webview: AppWebView,
    pub url_input: String,
}

impl WebViewState {
    pub fn new(default_url: impl Into<String>) -> (Self, AppTask) {
        let default_url = default_url.into();

        let res = Self {
            webview: WebView::new()
                .on_action(|action| WebViewModeMessage::WebView(action).into())
                .on_create_view(WebViewModeMessage::WebViewCreatedView.into())
                .on_url_change(|new_url| WebViewModeMessage::UpdateUrlInput(new_url).into()),

            url_input: default_url.clone(),
        };
        let task = AppTask::done(
            WebViewModeMessage::WebView(iced_webview::Action::CreateView(PageType::Url(
                default_url,
            )))
            .into(),
        );

        (res, task)
    }

    pub fn created_view(&mut self) -> AppTask {
        self.url_input = self.webview.current_url().into();
        AppTask::done(WebViewModeMessage::WebView(iced_webview::Action::ChangeView(0)).into())
    }

    pub fn action(&mut self, mut action: iced_webview::Action, config: &WebViewConfig) -> AppTask {
        if let iced_webview::Action::SendMouseEvent(mouse::Event::WheelScrolled { delta }, _) =
            &mut action
        {
            match delta {
                mouse::ScrollDelta::Lines { y, .. } => {
                    *y *= config.scroll_acceleration;
                }
                mouse::ScrollDelta::Pixels { y, .. } => {
                    *y *= config.scroll_acceleration;
                }
            }
        }

        self.webview.update(action)
    }

    pub fn update_url_input(&mut self, new_input: impl Into<String>) {
        self.url_input = new_input.into();
    }

    pub fn refresh(&mut self) -> AppTask {
        AppTask::done(WebViewModeMessage::WebView(iced_webview::Action::Refresh).into())
    }

    pub fn subscription(&self) -> AppSubscription {
        self.webview
            .subscription()
            .map(WebViewModeMessage::WebView)
            .map(Into::into)
    }
}
