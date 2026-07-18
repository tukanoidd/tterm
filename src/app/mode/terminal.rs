pub mod components;

use iced::{
    Event, Length, keyboard, mouse,
    widget::{self, column, row, rule},
};
use iced_swdir_tree::DirectoryTreeEvent;
use tterm_macros::mode;
use uuid::Uuid;

use crate::{
    app::{
        AppElement, AppMsg, AppSubscription, AppTask,
        mode::{TTermMode, TTermModeVariant},
        state::{directory_tree::DirectoryTreeState, tabs::TabsState},
    },
    config::{
        common::SplitDirection, keybinds::MoveFocusDirection, presets::TabConfig,
        terminal::TerminalConfig,
    },
};

use components::{
    multiplex::{
        pane::{IdPaneMessage, PaneMessage},
        tab::Tab,
    },
    tab_bar::TabBar,
};

mode! {
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

            IcedEvent(Event),
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
                DirectoryTreeToggle [ @[Alt+Shift] + "E" ],
                ToWebView           [ @[Alt+Shift] + "B" ],
            ]
        };
        config: {
            terminal: TerminalConfig
        };
        state: {
            directory_tree: DirectoryTreeState,
            tabs: TabsState,
        };


        fn view_impl<'a>(
            self,
            _config: &'a Self::Config,
            Self::State {
                directory_tree,
                tabs,
                ..
            }: &'a Self::State,
        ) -> impl Into<AppElement<'a>> {
            let directory_tree_view = directory_tree.view();

            let tab_widget = tabs.view();
            let dir_tree_tab_widget = row(directory_tree_view.into_iter().chain([tab_widget]));

            column![
                TabBar::new(tabs, directory_tree).view(),
                rule::horizontal(2),
                dir_tree_tab_widget,
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(10)
        }

        fn update_impl<'a>(
            self,
            message: Self::Message,
            config: &'a Self::Config,
            state: &'a mut Self::State,
        ) -> AppTask {
            match message {
                Self::Message::Action(action) => {
                    match action {
                        TerminalModeAction::Tab(tab_action) => match tab_action {
                            TerminalModeTabAction::New(tab_config) => {
                                let (tab, task) = match Tab::builder()
                                    .terminal_config(&config.terminal)
                                    .keybinds_config(&config.keybinds)
                                    .maybe_tab_config(tab_config)
                                    .build()
                                {
                                    Ok(tab) => tab,
                                    Err(err) => {
                                        return AppTask::done(AppMsg::Error {
                                            message: err.to_string(),
                                            critical: true,
                                        });
                                    }
                                };

                                state.tabs.list.push(tab);

                                return AppTask::batch([
                                    AppTask::done(TerminalModeTabAction::Select(state.tabs.list.len() - 1).into()),
                                    task,
                                ]);
                            },
                            TerminalModeTabAction::CloseFocused => {
                                return AppTask::done(
                                    <TerminalMode as TTermMode>::Message::CloseTab(
                                        state.tabs.list[state.tabs.current].id,
                                    )
                                    .into(),
                                );
                            },
                            TerminalModeTabAction::Select(index) => {
                                let index = index.clamp(0, state.tabs.list.len().saturating_sub(1));

                                state.tabs.current = index;

                                if state.tabs.list.is_empty() {
                                    return AppTask::none();
                                }

                                let current_tab = &mut state.tabs.list[state.tabs.current];

                                state.tabs.rename_content = current_tab.name.clone().unwrap_or_default();

                                let Some((_, pane_state)) = current_tab
                                    .panes
                                    .get_mut(&current_tab.current_panes_type)
                                    .and_then(|panes| panes.focused_pane_mut())
                                else {
                                    return AppTask::none();
                                };

                                return AppTask::done(
                                    <TerminalMode as TTermMode>::Message::FocusPane(pane_state.id)
                                        .into(),
                                );
                            }
                            TerminalModeTabAction::FocusedToggleFloating => {
                                let Some(current_tab) = state.tabs.current_tab_mut() else {
                                    return AppTask::none();
                                };

                                return current_tab.toggle_floating(&config.terminal, &config.keybinds);
                            }
                            TerminalModeTabAction::FocusedTogglePaneStacking => {
                                let Some(current_tab) = state.tabs.current_tab_mut() else {
                                    return AppTask::none();
                                };

                                current_tab.toggle_stacking();
                            }
                            TerminalModeTabAction::ToggleRename => {
                                state.tabs.rename_mode = true;

                                return widget::operation::focus("rename-tab-editor");
                            }
                        },
                        TerminalModeAction::Pane(pane_action) => {
                            let Some(current_tab) = state.tabs.current_tab_mut() else {
                                return AppTask::none();
                            };

                            return match pane_action {
                                TerminalModePaneAction::SplitFocused(direction) => match current_tab.split_focused(
                                    direction,
                                    &config.terminal,
                                    &config.keybinds,
                                ) {
                                    Ok(task) => task,
                                    Err(err) => AppTask::done(AppMsg::Error {
                                        message: err.to_string(),
                                        critical: false,
                                    }),
                                }
                                TerminalModePaneAction::CloseFocused => {
                                    current_tab.close_focused_pane()
                                }
                                TerminalModePaneAction::MoveFocused(direction) => {
                                    current_tab.move_focused_pane(direction)
                                }
                            };
                        },
                        TerminalModeAction::General(general_action) => match general_action {
                            TerminalModeGeneralAction::Focus(direction) => {
                                let Some(tab) = state.tabs.current_tab_mut() else {
                                    return AppTask::none();
                                };

                                match tab.focus_pane_directional(direction) {
                                    Some(task) => return task,
                                    None => match direction {
                                        MoveFocusDirection::Left => {
                                            return AppTask::done(
                                                TerminalModeTabAction::Select(match state.tabs.current {
                                                    0 => state.tabs.list.len() - 1,
                                                    _ => state.tabs.current - 1,
                                                })
                                                .into(),
                                            );
                                        }
                                        MoveFocusDirection::Right => {
                                            return AppTask::done(
                                                TerminalModeTabAction::Select(
                                                    match state.tabs.current >= state.tabs.list.len() - 1 {
                                                        true => 0,
                                                        false => state.tabs.current + 1,
                                                    },
                                                )
                                                .into(),
                                            );
                                        }
                                        _ => {}
                                    },
                                }
                            }
                            TerminalModeGeneralAction::DirectoryTreeToggle => {
                                state.directory_tree.show = !state.directory_tree.show;

                                if let Some((_, pane)) =
                                    state.tabs.current_tab().and_then(|t| t.focused_pane())
                                {
                                    return AppTask::done(
                                        <TerminalMode as TTermMode>::Message::FocusPane(pane.id).into(),
                                    );
                                }
                            }
                            TerminalModeGeneralAction::ToWebView => {
                                return AppTask::done(AppMsg::SwitchMode(TTermModeVariant::WebView));
                            }
                        }
                    }
                },
                Self::Message::PanelToggle { ty, force } => {
                    state.panel_toggle(ty, force);
                },
                Self::Message::UpdateFocusedDirectoryTree => {
                    let Some((_, focused_pane)) = state.tabs.focused_pane() else {
                        return AppTask::none();
                    };

                    return state.directory_tree.update_path(focused_pane);
                }
                Self::Message::DirectoryTree(event) => {
                    return state.directory_tree.update(event);
                }

                Self::Message::RenameTabInput(new_input) => {
                    state.tabs.rename_input(new_input);
                }
                Self::Message::RenameCurrentTab(new_name) => {
                    let new_name = new_name.trim();

                    if new_name.is_empty() {
                        return AppTask::none();
                    }

                    return state.tabs.rename_current_tab(new_name);
                }
                Self::Message::CloseTab(id) => {
                    return state.tabs.close(id);
                }
                Self::Message::TabResetFloating(id) => {
                    return state.tabs.reset_floating(id);
                }
                Self::Message::FocusPane(id) => {
                    return state.tabs.focus_pane(id);
                }

                Self::Message::Pane(pane_msg) => {
                    return state.tabs.update_pane(pane_msg);
                }

                Self::Message::IcedEvent(event) => match event {
                    iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(keyboard::key::Named::Escape),
                        repeat: false,
                        ..
                    }) => {
                        if state.tabs.rename_mode {
                            state.tabs.rename_mode = false;
                            return AppTask::done(TerminalModeTabAction::Select(state.tabs.current).into());
                        }
                    },
                iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                    if state.tabs.rename_mode {
                        return AppTask::none();
                    }

                    let Some(current_tab) = state.tabs.current_tab() else {
                        return AppTask::none();
                    };

                    let Some((_, focused_pane)) = current_tab.focused_pane() else {
                        return AppTask::none();
                    };

                    let scroll_delta = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => {
                            (y * config.terminal.scroll_acceleration) as i32
                        }
                        mouse::ScrollDelta::Pixels { y, .. } => {
                            (y * config.terminal.scroll_acceleration / config.terminal.font.size)
                                as i32
                        }
                    };

                    if scroll_delta != 0 {
                        return AppTask::done(
                            <TerminalMode as TTermMode>::Message::Pane(IdPaneMessage {
                                id: focused_pane.id,
                                msg: PaneMessage::TerminalMsg(iced_term::Event::BackendCall(
                                    focused_pane.term_id,
                                    iced_term::BackendCommand::Scroll(scroll_delta),
                                )),
                            })
                            .into(),
                        );
                    }
                }
                _ => {}
                },
            }

            AppTask::none()
        }

        fn subscription_impl<'a>(
            self,
            _config: &'a Self::Config,
            state: &'a Self::State,
        ) -> crate::app::AppSubscription {
            AppSubscription::batch(state.tabs.list.iter().map(Tab::subscription))
        }
    }
}
