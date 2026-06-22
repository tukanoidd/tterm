use std::collections::HashMap;

use iced::{
    Alignment, Length, Padding,
    widget::{button, center, container, row, table, text},
};
use iced_aw::DropDown;
use itertools::Itertools;
use strum::VariantArray;

use crate::{
    app::{AppElement, AppMsg},
    config::keybinds::{KeyBind, KeyBindPanelType, KeyBindsConfig, TTermAction},
};

pub struct KeyBindBar<'a> {
    keybinds_config: &'a KeyBindsConfig,
    keybind_panel_expanded: &'a HashMap<KeyBindPanelType, bool>,
}

impl<'a> KeyBindBar<'a> {
    pub fn new(
        keybinds_config: &'a KeyBindsConfig,
        keybind_panel_expanded: &'a HashMap<KeyBindPanelType, bool>,
    ) -> Self {
        Self {
            keybinds_config,
            keybind_panel_expanded,
        }
    }

    pub fn view(self) -> AppElement<'a> {
        let Self {
            keybinds_config,
            keybind_panel_expanded,
        } = self;

        const PADDING: u32 = 5;
        const SPACING: u32 = 5;

        let panels = <KeyBindPanelType as VariantArray>::VARIANTS
            .iter()
            .map(|ty| Self::panel(*ty, &keybinds_config.actions, keybind_panel_expanded))
            .collect::<Vec<_>>();

        center(
            row(panels)
                .padding(
                    Padding::default()
                        .bottom(PADDING)
                        .left(PADDING)
                        .right(PADDING),
                )
                .spacing(SPACING)
                .wrap(),
        )
        .height(Length::Shrink)
        .into()
    }

    fn panel(
        ty: KeyBindPanelType,
        binds: &'a HashMap<KeyBindPanelType, HashMap<KeyBind, TTermAction>>,
        keybind_panel_expanded: &'a HashMap<KeyBindPanelType, bool>,
    ) -> AppElement<'a> {
        let binds = binds
            .get(&ty)
            .iter()
            .flat_map(|b| b.iter())
            .sorted_by_key(|(_, action)| action.to_string());
        let table = table(
            [
                table::column(text("Binding"), |(bind, _): (&KeyBind, &TTermAction)| {
                    text(bind.to_string())
                }),
                table::column(text("Action"), |(_, action): (&KeyBind, &TTermAction)| {
                    text(action.to_string())
                }),
            ],
            binds,
        );

        const WIDTH: f32 = 400.0;

        DropDown::new(
            button(text(ty.title()).align_x(Alignment::Center))
                .style(button::subtle)
                .on_press(AppMsg::PanelToggle { ty, force: None })
                .width(Length::Fixed(WIDTH)),
            center(table).padding(5).style(container::bordered_box),
            keybind_panel_expanded.get(&ty).copied().unwrap_or_default(),
        )
        .width(Length::Fixed(WIDTH))
        .on_dismiss(AppMsg::PanelToggle {
            ty,
            force: Some(false),
        })
        .alignment(iced_aw::core::alignment::Alignment::Top)
        .into()
    }
}
