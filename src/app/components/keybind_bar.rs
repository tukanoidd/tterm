use std::collections::HashMap;

use iced::{
    Padding,
    widget::{button, container, row, table, text},
};
use iced_aw::DropDown;
use itertools::Itertools;
use strum::VariantArray;

use crate::{
    app::{AppElement, AppMsg, KeyBindPanelType},
    config::keybinds::{KeyBind, KeyBindsConfig, TTermAction},
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

        let panels = <KeyBindPanelType as VariantArray>::VARIANTS
            .iter()
            .map(|ty| Self::panel(*ty, &keybinds_config.actions, keybind_panel_expanded));

        row(panels)
            .padding(Padding::default().bottom(5).left(5).right(5))
            .wrap()
            .into()
    }

    fn panel(
        ty: KeyBindPanelType,
        binds: impl IntoIterator<Item = (&'a KeyBind, &'a TTermAction)>,
        keybind_panel_expanded: &'a HashMap<KeyBindPanelType, bool>,
    ) -> AppElement<'a> {
        let binds = binds
            .into_iter()
            .filter(|(_, a)| KeyBindPanelType::from(*a) == ty);
        let table = table(
            [
                table::column(text("Binding"), |(bind, _): (&KeyBind, &TTermAction)| {
                    text(bind.to_string())
                }),
                table::column(text("Action"), |(_, action): (&KeyBind, &TTermAction)| {
                    text(action.to_string())
                }),
            ],
            binds
                .into_iter()
                .sorted_by_key(|(_, action)| action.to_string()),
        )
        .width(350);

        DropDown::new(
            button(text(ty.title()))
                .style(button::subtle)
                .width(350)
                .on_press(AppMsg::PanelToggle { ty, force: None }),
            container(table).style(container::bordered_box),
            keybind_panel_expanded.get(&ty).copied().unwrap_or_default(),
        )
        .on_dismiss(AppMsg::PanelToggle {
            ty,
            force: Some(false),
        })
        .alignment(iced_aw::core::alignment::Alignment::Top)
        .into()
    }
}
