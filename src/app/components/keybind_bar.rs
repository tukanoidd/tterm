use std::collections::HashMap;

use iced::{
    Alignment, Length, Padding,
    alignment::Vertical,
    widget::{button, center, row, table, text},
};
use iced_aw::DropDown;
use iced_fonts::lucide;
use itertools::Itertools;
use strum::VariantArray;

use crate::{
    app::{AppElement, AppMsg},
    config::keybinds::KeyBind,
    fonts,
};

// pub struct KeyBindBar<'a> {
//     keybinds_config: &'a KeyBindsConfig,
//     keybind_panel_expanded: &'a HashMap<KeyBindPanelType, bool>,
// }

// impl<'a> KeyBindBar<'a> {
//     pub fn new(
//         keybinds_config: &'a KeyBindsConfig,
//         keybind_panel_expanded: &'a HashMap<KeyBindPanelType, bool>,
//     ) -> Self {
//         Self {
//             keybinds_config,
//             keybind_panel_expanded,
//         }
//     }

//     pub fn view(self) -> AppElement<'a> {
//         let Self {
//             keybinds_config,
//             keybind_panel_expanded,
//         } = self;

//         const PADDING: u32 = 5;
//         const SPACING: u32 = 5;

//         let panels = <KeyBindPanelType as VariantArray>::VARIANTS
//             .iter()
//             .map(|ty| Self::panel(*ty, &keybinds_config.actions, keybind_panel_expanded))
//             .collect::<Vec<_>>();

//         center(
//             row(panels)
//                 .padding(
//                     Padding::default()
//                         .bottom(PADDING)
//                         .left(PADDING)
//                         .right(PADDING),
//                 )
//                 .spacing(SPACING)
//                 .wrap(),
//         )
//         .height(Length::Shrink)
//         .into()
//     }

//     fn panel(
//         ty: KeyBindPanelType,
//         binds: &'a [(KeyBind, TTermAction)],
//         keybind_panel_expanded: &'a HashMap<KeyBindPanelType, bool>,
//     ) -> AppElement<'a> {
//         let binds = binds
//             .iter()
//             .filter(|(_, action)| {
//                 matches!(
//                     (action, ty),
//                     (TTermAction::Tab(_), KeyBindPanelType::Tab)
//                         | (TTermAction::Pane(_), KeyBindPanelType::Pane)
//                         | (TTermAction::General(_), KeyBindPanelType::General),
//                 )
//             })
//             .sorted_by_key(|(_, action)| action.to_string());
//         let table = table(
//             [
//                 table::column(
//                     text("Binding").center(),
//                     |(bind, _): &(KeyBind, TTermAction)| text(bind.to_string()).center(),
//                 ),
//                 table::column(text("Action"), |(_, action): &(KeyBind, TTermAction)| {
//                     text(action.to_string()).center()
//                 }),
//             ],
//             binds,
//         )
//         .width(Length::Fill);

//         let expanded = keybind_panel_expanded.get(&ty).copied().unwrap_or_default();

//         let panel_button_icon = match expanded {
//             true => lucide::arrow_up_from_line(),
//             false => lucide::arrow_down_from_line(),
//         };
//         let panel_button = button(
//             center(
//                 row![
//                     panel_button_icon,
//                     text(ty.title())
//                         .align_x(Alignment::Center)
//                         .font(fonts::MONOSPACE_ROBOTO_MONO_NERD_FONT_MONO_BOLD_FONT)
//                 ]
//                 .align_y(Vertical::Center)
//                 .spacing(6)
//                 .padding(Padding::default().horizontal(150)),
//             )
//             .width(Length::Shrink)
//             .height(Length::Shrink),
//         )
//         .style(style::panel_button(expanded))
//         .on_press(AppMsg::PanelToggle { ty, force: None });

//         let panel = center(table).padding(5).style(style::panel);

//         DropDown::new(panel_button, panel, expanded)
//             .on_dismiss(AppMsg::PanelToggle {
//                 ty,
//                 force: Some(false),
//             })
//             .alignment(iced_aw::core::alignment::Alignment::Top)
//             .into()
//     }
// }

// pub mod style {
//     use iced::widget::{button, container};

//     use crate::app::AppTheme;

//     pub fn panel_button(expanded: bool) -> impl Fn(&AppTheme, button::Status) -> button::Style {
//         move |theme, status| {
//             let status = match expanded {
//                 true => button::Status::Hovered,
//                 false => status,
//             };

//             let mut style = button::subtle(theme, status);
//             style.border = style.border.rounded(20);

//             style
//         }
//     }

//     pub fn panel(theme: &AppTheme) -> container::Style {
//         let style = container::bordered_box(theme);
//         style.border(style.border.rounded(20))
//     }
// }
