use iced::{
    Length, Padding,
    widget::{container, mouse_area, row, scrollable, text},
};
use iced_aw::badge;

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

        container(scrollable(
            row(tabs.iter().enumerate().map(|(ind, Tab { name, .. })| {
                mouse_area(
                    badge(text(match name {
                        Some(name) => name.clone(),
                        None => format!("Tab #{ind}"),
                    }))
                    .style(move |theme, status| match current_tab == ind {
                        true => iced_aw::style::badge::primary(theme, status),
                        false => iced_aw::style::badge::secondary(theme, status),
                    }),
                )
                .on_press(AppMsg::SelectTab(ind))
                .into()
            }))
            .spacing(5),
        ))
        .padding(Padding::default().top(5).horizontal(5))
        .width(Length::Fill)
        .into()
    }
}
