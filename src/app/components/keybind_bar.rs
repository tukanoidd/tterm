use iced::{
    Padding,
    widget::{button, container, row, table, text},
};
use iced_aw::DropDown;
use itertools::Itertools;

use crate::{
    app::{AppElement, AppMsg},
    config::keybinds::{KeyBind, KeyBindsConfig, TTermAction},
};

pub struct KeyBindBar<'a> {
    keybinds_config: &'a KeyBindsConfig,
    tab_expanded: bool,
    pane_expanded: bool,
    general_expanded: bool,
}

impl<'a> KeyBindBar<'a> {
    pub fn new(
        keybinds_config: &'a KeyBindsConfig,
        tab_expanded: bool,
        pane_expanded: bool,
        general_expanded: bool,
    ) -> Self {
        Self {
            keybinds_config,
            tab_expanded,
            pane_expanded,
            general_expanded,
        }
    }

    pub fn view(self) -> AppElement<'a> {
        let Self {
            keybinds_config,
            tab_expanded,
            pane_expanded,
            general_expanded,
        } = self;

        macro_rules! panel {
            (
                [
                    $str_name:literal
                    | $expanded:expr
                    => $toggle:ident
                ]
                [
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
                    $str_name,
                    actions,
                    $expanded,
                    AppMsg::$toggle,
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
            ["Tab Actions" | tab_expanded => TabPanelToggle]
            [NewTab | CloseFocusedTab | SelectTab @(_t)]
        );
        let pane_panel = panel!(
            ["Pane Actions" | pane_expanded => PanePanelToggle]
            [SplitPaneVertical | SplitPaneHorizontal | CloseFocusedPane]
        );
        let general_panel = panel!(
            ["General Actions" | general_expanded => GeneralPanelToggle]
            [FocusLeft | FocusRight | FocusUp | FocusDown]
        );

        row([tab_panel, pane_panel, general_panel])
            .padding(Padding::default().bottom(5).left(5).right(5))
            .wrap()
            .into()
    }

    fn panel(
        name: &'a str,
        binds: impl IntoIterator<Item = (&'a KeyBind, &'a TTermAction)>,
        expanded: bool,
        on_toggle: AppMsg,
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
            button(text(name))
                .style(button::subtle)
                .width(350)
                .on_press(on_toggle.clone()),
            container(table).style(container::bordered_box),
            expanded,
        )
        .on_dismiss(on_toggle)
        .alignment(iced_aw::core::alignment::Alignment::Top)
        .into()
    }
}
