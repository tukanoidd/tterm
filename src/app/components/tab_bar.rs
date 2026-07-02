use bon::bon;
use iced::{
    Length, Padding,
    alignment::Vertical,
    widget::{
        button, center, column, container, mouse_area, rich_text, row, rule, scrollable, space,
        span, text, text_editor,
    },
};
use iced_aw::{Badge, ContextMenu, badge};
use iced_fonts::lucide;

use crate::{
    app::{AppElement, AppMsg, AppRenderer, AppTheme},
    config::keybinds::{TTermGeneralAction, TTermTabAction},
    fonts,
    multiplex::tab::Tab,
};

pub struct TabBar<'a> {
    tabs: &'a [Tab],
    current_tab: usize,

    rename_mode: bool,
    rename_content: &'a text_editor::Content,

    show_directory_tree: bool,
}

#[bon]
impl<'a> TabBar<'a> {
    pub fn new(
        tabs: &'a [Tab],
        current_tab: usize,
        rename_mode: bool,
        rename_content: &'a text_editor::Content,
        show_directory_tree: bool,
    ) -> Self {
        Self {
            tabs,
            current_tab,

            rename_mode,
            rename_content,

            show_directory_tree,
        }
    }

    pub fn view(self) -> AppElement<'a> {
        let Self {
            tabs,
            current_tab,

            rename_mode,
            rename_content,

            show_directory_tree,
        } = self;

        let toggle_show_directory_tree_button = button(match show_directory_tree {
            true => lucide::panel_left_open(),
            false => lucide::panel_left_close(),
        })
        .style(button::subtle)
        .on_press(TTermGeneralAction::DirectoryTreeToggle.into());

        let scrollable_tab_list = Self::tab_list(tabs, current_tab, rename_mode);
        let current_tab_name_editor = rename_mode.then(|| {
            text_editor(rename_content)
                .id("rename-tab-editor")
                .on_action(AppMsg::RenameTabEditorAction)
        });

        container(
            row([
                toggle_show_directory_tree_button.into(),
                space().width(Length::Fixed(15.0)).into(),
                scrollable_tab_list,
            ]
            .into_iter()
            .chain(
                current_tab_name_editor
                    .map(|ed| [space().width(Length::Fill).into(), ed.width(300).into()])
                    .into_iter()
                    .flatten(),
            ))
            .align_y(Vertical::Center),
        )
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
            .align_y(Vertical::Center)
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
        move |(ind, Tab { id, name, .. })| {
            const ICON_SIZE: f32 = 30.0;

            let is_current = current_tab == ind;
            let icon = match is_current {
                true => match rename_mode {
                    true => lucide::square_pen(),
                    false => lucide::focus(),
                },
                false => lucide::scan(),
            }
            .center()
            .width(Length::Fixed(ICON_SIZE))
            .height(Length::Fixed(ICON_SIZE));

            let name_text = match name {
                Some(name) => name.clone(),
                None => format!("Tab #{ind}"),
            };

            const CLOSE_BUTTON_SIZE: f32 = 25.0;

            let close_button = button(center(lucide::x().center()))
                .padding(2)
                .width(Length::Fixed(CLOSE_BUTTON_SIZE))
                .height(Length::Fixed(CLOSE_BUTTON_SIZE))
                .style(style::close_button)
                .on_press(AppMsg::CloseTab(*id));
            let badge = Self::badge()
                .icon(icon)
                .content(name_text)
                .underline(is_current)
                .additional(close_button)
                .call()
                .style(style::tab_badge(current_tab, ind))
                .padding(2);

            ContextMenu::new(
                mouse_area(badge).on_press(TTermTabAction::Select(ind).into()),
                || {
                    column(
                        TTermTabAction::default_keybinds()
                            .into_iter()
                            .filter(|(_, action)| {
                                matches!(
                                    action,
                                    TTermTabAction::CloseFocused
                                        | TTermTabAction::FocusedToggleFloating
                                        | TTermTabAction::FocusedTogglePaneStacking
                                        | TTermTabAction::ToggleRename
                                )
                            })
                            .map(|(_, action)| {
                                button(text(action.to_string()))
                                    .on_press(action.into())
                                    .width(Length::Fixed(275.0))
                                    .style(button::subtle)
                                    .into()
                            }),
                    )
                    .into()
                },
            )
            .into()
        }
    }

    fn new_tab_badge() -> AppElement<'a> {
        mouse_area(
            Self::badge()
                .icon(lucide::plus())
                .content("New Tab")
                .call()
                .style(iced_aw::style::badge::secondary)
                .padding(6),
        )
        .on_press(TTermTabAction::New(None).into())
        .into()
    }

    #[builder]
    fn badge(
        #[builder(into)] icon: AppElement<'a>,
        content: impl text::IntoFragment<'a>,
        #[builder(default)] underline: bool,
        #[builder(into)] additional: Option<AppElement<'a>>,
    ) -> Badge<'a, AppMsg, AppTheme, AppRenderer> {
        badge(
            row([
                icon,
                rich_text::<'_, (), _, _, _>([span(content).underline(underline)])
                    .font(fonts::MONOSPACE_ROBOTO_MONO_NERD_FONT_MONO_BOLD_FONT)
                    .center()
                    .into(),
            ]
            .into_iter()
            .chain(additional))
            .align_y(Vertical::Center)
            .padding(Padding::default().vertical(2).horizontal(2))
            .spacing(8),
        )
    }
}

pub mod style {
    use iced::widget::button;
    use iced_aw::badge;

    use crate::app::AppTheme;

    pub fn tab_badge(
        current_tab: usize,
        ind: usize,
    ) -> impl Fn(&AppTheme, badge::Status) -> badge::Style {
        move |theme, status| match current_tab == ind {
            true => iced_aw::style::badge::info(theme, status),
            false => iced_aw::style::badge::primary(theme, status),
        }
    }

    pub fn close_button(theme: &AppTheme, status: button::Status) -> button::Style {
        let mut style = button::secondary(theme, status);
        style.border = style.border.rounded(25);

        style
    }
}
