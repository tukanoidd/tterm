use iced::{
    Length, Padding,
    alignment::Vertical,
    widget::{container, mouse_area, row, rule, scrollable, space, text, text_editor},
};
use iced_aw::{Badge, badge};
use iced_fonts::lucide;

use crate::{
    app::{AppElement, AppMsg, AppRenderer, AppTheme},
    config::keybinds::TTermTabAction,
    fonts,
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

        let scrollable_tab_list = Self::tab_list(tabs, current_tab, rename_mode);
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

    fn tab_list(tabs: &'a [Tab], current_tab: usize, rename_mode: bool) -> AppElement<'a> {
        scrollable(
            row(tabs
                .iter()
                .enumerate()
                .map(Self::tab_badge(current_tab, rename_mode))
                .chain([rule::vertical(2).into(), Self::new_tab_badge()]))
            .height(Length::Shrink)
            .spacing(10),
        )
        .horizontal()
        .into()
    }

    fn tab_badge(
        current_tab: usize,
        rename_mode: bool,
    ) -> impl Fn((usize, &Tab)) -> AppElement<'a> {
        move |(ind, Tab { name, .. })| {
            let icon = match current_tab == ind {
                true => match rename_mode {
                    true => lucide::square_pen(),
                    false => lucide::focus(),
                },
                false => lucide::scan(),
            };
            let name_text = match name {
                Some(name) => name.clone(),
                None => format!("Tab #{ind}"),
            };
            let badge = Self::badge(icon, name_text).style(style::tab_badge(current_tab, ind));

            mouse_area(badge)
                .on_press(TTermTabAction::Select(ind).into())
                .into()
        }
    }

    fn new_tab_badge() -> AppElement<'a> {
        mouse_area(Self::badge(lucide::plus(), "New Tab").style(iced_aw::style::badge::secondary))
            .on_press(TTermTabAction::New(None).into())
            .into()
    }

    fn badge(
        icon: impl Into<AppElement<'a>>,
        content: impl text::IntoFragment<'a>,
    ) -> Badge<'a, AppMsg, AppTheme, AppRenderer> {
        badge(
            row![
                icon.into(),
                text(content).font(fonts::MONOSPACE_ROBOTO_MONO_NERD_FONT_MONO_BOLD_FONT)
            ]
            .align_y(Vertical::Center)
            .padding(2)
            .spacing(6),
        )
    }
}

pub mod style {
    use iced_aw::badge;

    pub fn tab_badge(
        current_tab: usize,
        ind: usize,
    ) -> impl Fn(&iced::Theme, badge::Status) -> badge::Style {
        move |theme, status| match current_tab == ind {
            true => iced_aw::style::badge::info(theme, status),
            false => iced_aw::style::badge::primary(theme, status),
        }
    }
}
