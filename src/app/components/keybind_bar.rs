use std::collections::HashMap;

use iced::{
    Padding,
    widget::{button, container, row, table, text},
};
use iced_aw::DropDown;
use itertools::Itertools;

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

        macro_rules! panel {
            (
                $panel_ty:ident: [
                    $(
                        $action:ident
                        $(@($($tp:ident),*))?
                        $(@{$sp:ident})?
                    )|+
                ]
            ) => {{
                let actions = keybinds_config.actions.iter().filter(|(_, action)| {
                    matches!(
                        action,
                        $(panel!(@act $action $(@($($tp),*))? $(@{$sp})?))|+
                    )
                });

                Self::panel(
                    KeyBindPanelType::$panel_ty,
                    actions,
                    keybind_panel_expanded
                )
            }};

            (@act $name:ident) => {
                TTermAction::$name
            };
            (@act $name:ident @($($p:ident),*)) => {
                TTermAction::$name($(panel!(@act @tup $p)),*)
            };
            (@act @tup $tup_var:ident) => { _ };
            (@act $name:ident @{$($p:ident)?}) => {
                TTermAction::$name {..}
            };
        }

        let tab_panel = panel!(
            Tab: [NewTab | CloseFocusedTab | SelectTab @(_t)]
        );
        let pane_panel = panel!(
            Pane: [SplitFocusedPane @(_d) | CloseFocusedPane]
        );
        let general_panel = panel!(
            General: [Focus @(_d)]
        );

        row([tab_panel, pane_panel, general_panel])
            .padding(Padding::default().bottom(5).left(5).right(5))
            .wrap()
            .into()
    }

    fn panel(
        ty: KeyBindPanelType,
        binds: impl IntoIterator<Item = (&'a KeyBind, &'a TTermAction)>,
        keybind_panel_expanded: &'a HashMap<KeyBindPanelType, bool>,
    ) -> AppElement<'a> {
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
