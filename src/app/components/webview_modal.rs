use iced::{
    Length, Padding,
    widget::{center, column, container, text_input},
};
use iced_webview::WebView;

use crate::app::{AppElement, AppMsg, AppTheme, state::webview::WebViewState};

pub type WebViewEngine = iced_webview::Servo;
pub type AppWebView = WebView<WebViewEngine, AppMsg>;

pub struct WebViewModal<'a> {
    state: &'a WebViewState,
}

impl<'a> WebViewModal<'a> {
    pub fn new(state: &'a WebViewState) -> Self {
        Self { state }
    }

    pub fn view(self) -> Option<AppElement<'a>> {
        let Self { state } = self;

        state.show.then(|| {
            center(
                column![
                    center(
                        text_input("Enter url...", &state.url_input)
                            .on_input(AppMsg::UpdateUrlInput)
                            .on_submit(AppMsg::from_result(
                                url::Url::parse(&state.url_input)
                                    .map(iced_webview::Action::GoToUrl),
                                Into::into,
                                false
                            ),)
                    )
                    .height(Length::Shrink)
                    .padding(Padding::default().horizontal(5)),
                    state.webview.view().map(AppMsg::WebView)
                ]
                .width(Length::Fill)
                .height(Length::Fill)
                .spacing(5),
            )
            .padding(15.0)
            .style(|theme: &AppTheme| {
                container::background(theme.palette().background.scale_alpha(0.5))
            })
            .into()
        })
    }
}
