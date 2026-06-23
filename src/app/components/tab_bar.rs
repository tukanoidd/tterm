use iced::{
    Length, Padding,
    alignment::Vertical,
    widget::{container, mouse_area, row, rule, scrollable, space, text, text_editor},
};
use iced_aw::badge;
use iced_fonts::lucide;

use crate::{
    app::{AppElement, AppMsg},
    config::keybinds::TTermTabAction,
    multiplex::tab::Tab,
};

pub struct TabBar<'a> {
    tabs: &'a [Tab],
    current_tab: usize,

    rename_mode: bool,
    rename_content: &'a text_editor::Content,
}

impl<'a> TabBar<'a> {
    pub fn new(
        tabs: &'a [Tab],
        current_tab: usize,
        rename_mode: bool,
        rename_content: &'a text_editor::Content,
    ) -> Self {
        Self {
            tabs,
            current_tab,

            rename_mode,
            rename_content,
        }
    }

    pub fn view(self) -> AppElement<'a> {
        let Self {
            tabs,
            current_tab,

            rename_mode,
            rename_content,
        } = self;

        let scrollable_tab_list = Self::tab_list(tabs, current_tab);
        let current_tab_name_editor = rename_mode.then(|| {
            text_editor(rename_content)
                .id("rename-tab-editor")
                .on_action(AppMsg::RenameTabEditorAction)
        });

        container(row([scrollable_tab_list].into_iter().chain(
            current_tab_name_editor
                .map(|ed| [space().width(Length::Fill).into(), ed.width(300).into()])
                .into_iter()
                .flatten(),
        )))
        .padding(Padding::default().top(5).horizontal(5))
        .width(Length::Fill)
        .into()
    }

    fn tab_list(tabs: &'a [Tab], current_tab: usize) -> AppElement<'a> {
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
                    .on_press(TTermTabAction::Select(ind).into())
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
                    .on_press(TTermTabAction::New(None).into())
                    .into(),
                ]))
            .height(Length::Shrink)
            .spacing(10),
        )
        .horizontal()
        .into()
    }
}
