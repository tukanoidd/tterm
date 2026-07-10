use iced::{
    Length,
    widget::{center, column, row, rule, text},
};
use iced_swdir_tree::DirectoryTreeEvent;
use tterm_macros::modes;
use uuid::Uuid;

use crate::{
    app::{
        AppElement, AppMsg, AppSubscription, AppTask, MainState,
        components::tab_bar::TabBar,
        state::{directory_tree::DirectoryTreeState, tabs::TabsState},
    },
    config::{
        Config, common::SplitDirection, keybinds::MoveFocusDirection, presets::TabConfig,
        terminal::TerminalConfig, webview::WebViewConfig,
    },
    multiplex::pane::IdPaneMessage,
};

pub trait TTermMode<'a> {
    type Message: Into<AppMsg>;
    type Config;

    type ViewState: From<&'a MainState>;
    type UpdateState: From<&'a mut MainState>;
    type SubscriptionState: From<&'a MainState>;

    #[inline]
    fn view(self, config: &'a Config, state: &'a MainState) -> AppElement<'a>
    where
        Self: Sized + 'static,
        Config: AsRef<Self::Config>,
    {
        self.view_impl(config.as_ref(), state.into()).into()
    }

    fn view_impl(
        self,
        config: &'a Self::Config,
        state: Self::ViewState,
    ) -> impl Into<AppElement<'a>>;

    #[inline]
    fn update(self, message: Self::Message, config: &'a Config, state: &'a mut MainState) -> AppTask
    where
        Self: Sized + 'static,
        Config: AsRef<Self::Config>,
    {
        self.update_impl(message, config.as_ref(), state.into())
    }

    fn update_impl(
        self,
        message: Self::Message,
        config: &'a Self::Config,
        state: Self::UpdateState,
    ) -> AppTask;

    #[inline]
    fn subscription(self, config: &'a Config, state: &'a MainState) -> AppSubscription
    where
        Self: Sized + 'static,
        Config: AsRef<Self::Config>,
    {
        self.subscription_impl(config.as_ref(), state.into())
    }

    fn subscription_impl(
        self,
        config: &'a Self::Config,
        state: Self::SubscriptionState,
    ) -> AppSubscription;
}

#[macro_export]
macro_rules! mode_state {
    (@view {$($field:ident: $ty:ty),+ $(,)?}) => {
        pub struct ViewState<'a> {
            $($field: &'a $ty),+
        }

        impl<'a> From<&'a $crate::app::MainState>  for ViewState<'a> {
            fn from($crate::app::MainState {
                $($field,)+
                ..
            }: &'a $crate::app::MainState) -> Self {
                Self {
                    $($field),+
                }
            }
        }
    };
    (@update {$($field:ident: $ty:ty),+ $(,)?}) => {
        pub struct UpdateState<'a> {
            $($field: &'a mut $ty),+
        }

        impl<'a> From<&'a mut $crate::app::MainState> for UpdateState<'a> {
            fn from($crate::app::MainState {
                $($field,)+
                ..
            }: &'a mut $crate::app::MainState) -> Self {
                Self {
                    $($field),+
                }
            }
        }
    };
    (@subscription {$($field:ident: $ty:ty),+ $(,)?}) => {
        pub struct SubscriptionState<'a> {
            $($field: &'a $ty),+
        }

        impl<'a> From<&'a $crate::app::MainState> for SubscriptionState<'a> {
            fn from($crate::app::MainState {
                $($field,)+
                ..
            }: &'a $crate::app::MainState) -> Self {
                Self {
                    $($field),+
                }
            }
        }
    };
}

modes! {
    Terminal: {
        message: {
            UpdateFocusedDirectoryTree,
            DirectoryTree(DirectoryTreeEvent),

            #[from(skip)]
            RenameTabInput(String),
            #[from(skip)]
            RenameCurrentTab(String),
            #[from(skip)]
            CloseTab(Uuid),
            #[from(skip)]
            TabResetFloating(Uuid),
            #[from(skip)]
            FocusPane(Uuid),

            Pane(IdPaneMessage),
        };
        actions: {
            Tab: [
                #[display("New Tab")]
                New(Option<TabConfig>) [ @[Ctrl+Shift] + "T" => (None) ],
                #[display("Close Tab")]
                CloseFocused           [ @[Ctrl+Shift] + "W" ],
                #[display("Select Tab {_0}")]
                Select(usize)          [
                    @[Ctrl+Shift] + "1" => (0),
                    @[Ctrl+Shift] + "2" => (1),
                    @[Ctrl+Shift] + "3" => (2),
                    @[Ctrl+Shift] + "4" => (3),
                    @[Ctrl+Shift] + "5" => (4),
                    @[Ctrl+Shift] + "6" => (5),
                    @[Ctrl+Shift] + "7" => (6),
                    @[Ctrl+Shift] + "8" => (7),
                    @[Ctrl+Shift] + "9" => (8),
                ],
                #[display("Toggle Floating Panes")]
                FocusedToggleFloating     [ @[Ctrl+Shift] + "E" ],
                #[display("Toggle Pane stacking")]
                FocusedTogglePaneStacking [ @[Ctrl+Shift] + "S" ],
                ToggleRename              [ @[Ctrl+Shift] + "R" ],
            ],
            Pane: [
                #[display("Split Pane {}", match _0 {
                    SplitDirection::Vertical => "Vertically",
                    SplitDirection::Horizontal => "Horizontally"
                })]
                SplitFocused(SplitDirection) [
                    @[Alt] + "V" => (SplitDirection::Vertical),
                    @[Alt] + "H" => (SplitDirection::Horizontal),
                ],
                #[display("Close Focused Pane")]
                CloseFocused [ @[Alt] + "W" ],
                #[display("Move Pane {_0}")]
                MoveFocused(MoveFocusDirection) [
                    @[Alt+Shift] + @ArrowLeft => (MoveFocusDirection::Left),
                    @[Alt+Shift] + @ArrowRight => (MoveFocusDirection::Right),
                    @[Alt+Shift] + @ArrowUp => (MoveFocusDirection::Up),
                    @[Alt+Shift] + @ArrowDown => (MoveFocusDirection::Down),
                ],
            ],
            General: [
                #[display("Focus {_0}")]
                Focus(MoveFocusDirection) [
                    @[Alt] + @ArrowLeft => (MoveFocusDirection::Left),
                    @[Alt] + @ArrowRight => (MoveFocusDirection::Right),
                    @[Alt] + @ArrowUp => (MoveFocusDirection::Up),
                    @[Alt] + @ArrowDown => (MoveFocusDirection::Down),
                ],
                KeyBindPanelsToggle [ @[Alt+Shift] + "K" ],
                DirectoryTreeToggle [ @[Alt+Shift] + "E" ],
                ToWebView           [ @[Alt+Shift] + "B" ],
            ]
        };
        config: {
            terminal: TerminalConfig
        };
        states: [
            @view {
                directory_tree_state: DirectoryTreeState,
                tabs_state: TabsState,
            },
            @update {
                directory_tree_state: DirectoryTreeState,
                tabs_state: TabsState,

            },
            @subscription {
                directory_tree_state: DirectoryTreeState,
                tabs_state: TabsState,
            }
        ];


        fn view_impl(
            self,
            _config: &'a Self::Config,
            Self::ViewState {
                directory_tree_state,
                tabs_state,
            }: Self::ViewState,
        ) -> impl Into<AppElement<'a>> {
            let directory_tree_view = directory_tree_state.view();

            let tab_widget = tabs_state.view();
            let dir_tree_tab_widget = row(directory_tree_view.into_iter().chain([tab_widget]));

            column![
                TabBar::new(tabs_state, directory_tree_state).view(),
                rule::horizontal(2),
                dir_tree_tab_widget,
                // rule::horizontal(2),
                // KeyBindBar::new(&config.keybinds, panel_expanded).view()
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(10)
            .padding(5)
        }

        fn update_impl(
            self,
            message: Self::Message,
            _config: &'a Self::Config,
            Self::UpdateState {
                directory_tree_state,
                tabs_state,
            }: Self::UpdateState,
        ) -> AppTask {
            match message {
                Self::Message::Action(action) => {
                    match action {
                        TerminalModeAction::Tab(tab_action) => {},
                        TerminalModeAction::Pane(pane_action) => {},
                        TerminalModeAction::General(general_action) => {}
                    }
                },
                Self::Message::UpdateFocusedDirectoryTree => {
                    let Some((_, focused_pane)) = tabs_state.focused_pane() else {
                        return AppTask::none();
                    };

                    return directory_tree_state.update_path(focused_pane);
                }
                Self::Message::DirectoryTree(event) => {
                    return directory_tree_state.update(event);
                }

                Self::Message::RenameTabInput(new_input) => {
                    tabs_state.rename_input(new_input);
                }
                Self::Message::RenameCurrentTab(new_name) => {
                    let new_name = new_name.trim();

                    if new_name.is_empty() {
                        return AppTask::none();
                    }

                    return tabs_state.rename_current_tab(new_name);
                }
                Self::Message::CloseTab(id) => {
                    return tabs_state.close(id);
                }
                Self::Message::TabResetFloating(id) => {
                    return tabs_state.reset_floating(id);
                }
                Self::Message::FocusPane(id) => {
                    return tabs_state.focus_pane(id);
                }

                Self::Message::Pane(pane_msg) => {
                    return tabs_state.update_pane(pane_msg);
                }
            }

            AppTask::none()
        }

        fn subscription_impl(
            self,
            config: &'a Self::Config,
            state: Self::SubscriptionState,
        ) -> crate::app::AppSubscription {
            todo!()
        }
    },
    WebView: {
        message: {};
        actions: {
            Tab: [
                #[display("New Tab")]
                New(Option<TabConfig>) [ @[Ctrl+Shift] + "T" => (None) ],
                #[display("Close Tab")]
                CloseFocused           [ @[Ctrl+Shift] + "W" ],
                #[display("Select Tab {_0}")]
                Select(usize)          [
                    @[Ctrl+Shift] + "1" => (0),
                    @[Ctrl+Shift] + "2" => (1),
                    @[Ctrl+Shift] + "3" => (2),
                    @[Ctrl+Shift] + "4" => (3),
                    @[Ctrl+Shift] + "5" => (4),
                    @[Ctrl+Shift] + "6" => (5),
                    @[Ctrl+Shift] + "7" => (6),
                    @[Ctrl+Shift] + "8" => (7),
                    @[Ctrl+Shift] + "9" => (8),
                ],
                ToggleRename              [ @[Ctrl+Shift] + "R" ],
            ],
            General: [
                KeyBindPanelsToggle [ @[Alt+Shift] + "K" ],
                ToTerminal          [ @[Alt+Shift] + "B" ],
            ]
        };

        config: {
            base: WebViewConfig
        };

        states: [
            @view {},
            @update {},
            @subscription {}
        ];

        fn view_impl(
            self,
            config: &'a Self::Config,
            state: Self::ViewState
        ) -> impl Into<AppElement<'a>> {
            center(text("todo"))
        }

        fn update_impl(
            self,
            message: Self::Message,
            config: &'a mut Self::Config,
            state: Self::UpdateState
        ) -> AppTask {
            todo!()
        }

        fn subscription_impl(
            self,
            config: &'a Self::Config,
            state: Self::UpdateState
        ) -> AppSubscription {
            todo!()
        }
    },
}
