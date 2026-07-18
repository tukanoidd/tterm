use iced::{
    Length,
    alignment::Vertical,
    widget::{button, column, row, text_input},
};
use iced_fonts::lucide;
use iced_webview::WebView;

use crate::app::{
    AppElement, AppMsg,
    mode::webview::{WebViewModeGeneralAction, WebViewModeMessage},
    state::webview::WebViewState,
};

pub type WebViewEngine = iced_webview::Servo;
pub type AppWebView = WebView<WebViewEngine, AppMsg>;

pub struct WebViewModal<'a> {
    state: &'a WebViewState,
}

impl<'a> WebViewModal<'a> {
    pub fn new(state: &'a WebViewState) -> Self {
        Self { state }
    }

    pub fn view(self) -> AppElement<'a> {
        let Self { state } = self;

        column![
            row![
                button(lucide::square_terminal())
                    .on_press(WebViewModeGeneralAction::ToTerminal.into()),
                text_input("Enter url...", &state.url_input)
                    .on_input(|new_url| WebViewModeMessage::UpdateUrlInput(new_url).into())
                    .on_submit(AppMsg::from_result(
                        url::Url::parse(&state.url_input)
                            .map(iced_webview::Action::GoToUrl)
                            .map(WebViewModeMessage::WebView),
                        Into::into,
                        false
                    ))
            ]
            .align_y(Vertical::Center)
            .spacing(5)
            .height(Length::Shrink),
            state
                .webview
                .view()
                .map(WebViewModeMessage::WebView)
                .map(Into::into)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(5)
        .into()
    }
}
