use iced::{
    Length, Padding,
    alignment::Vertical,
    widget::{container, mouse_area, row, rule, scrollable, text},
};
use iced_aw::badge;
use iced_fonts::lucide;

use crate::{
    app::{AppElement, AppMsg},
    multiplex::tab::Tab,
};

pub struct TabBar<'a> {
    tabs: &'a [Tab],
    current_tab: usize,
}

impl<'a> TabBar<'a> {
    pub fn new(tabs: &'a [Tab], current_tab: usize) -> Self {
        Self { tabs, current_tab }
    }

    pub fn view(self) -> AppElement<'a> {
        let Self { tabs, current_tab } = self;

        container(
            scrollable(
                row(tabs
                    .iter()
                    .enumerate()
                    .map(|(ind, Tab { name, .. })| {
                        mouse_area(
                            badge(text(match name {
                                Some(name) => name.clone(),
                                None => format!("Tab #{ind}"),
                            }))
                            .style(move |theme, status| {
                                match current_tab == ind {
                                    true => iced_aw::style::badge::info(theme, status),
                                    false => iced_aw::style::badge::primary(theme, status),
                                }
                            }),
                        )
                        .on_press(AppMsg::SelectTab(ind))
                        .into()
                    })
                    .chain([
                        rule::vertical(2).into(),
                        mouse_area(
                            badge(
                                row![lucide::plus(), text("New Tab")]
                                    .align_y(Vertical::Center)
                                    .spacing(2),
                            )
                            .style(iced_aw::style::badge::secondary),
                        )
                        .on_press(AppMsg::NewTab)
                        .into(),
                    ]))
                .height(Length::Shrink)
                .spacing(10),
            )
            .horizontal(),
        )
        .padding(Padding::default().top(5).horizontal(5))
        .width(Length::Fill)
        .into()
    }
}
