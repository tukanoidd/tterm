use std::collections::HashMap;

use bon::bon;
use iced::{
    Length, Padding,
    widget::{
        Float, button, center, column, container, pane_grid, responsive, row, scrollable, space,
        stack,
    },
};
use iced_aw::Spinner;
use itertools::Itertools;
use rootcause::{Result, option_ext::OptionExt};
use uuid::Uuid;

use crate::{
    app::{AppElement, AppMsg, AppSubscription, AppTask},
    config::{
        common::SplitDirection,
        keybinds::{KeyBindsConfig, MoveFocusDirection},
        presets::{PaneConfig, PaneSplitConfig, TabConfig},
        terminal::TerminalConfig,
    },
    multiplex::pane::{IdPaneMessage, PaneState},
};

use super::pane::PaneMessage;

#[derive(Debug)]
pub struct Tab {
    pub id: Uuid,
    pub name: Option<String>,

    pub panes: HashMap<TabPanesType, TabPanesState>,
    pub current_panes_type: TabPanesType,
}

#[bon]
impl Tab {
    #[builder]
    pub fn new(
        terminal_config: &TerminalConfig,
        keybinds_config: &KeyBindsConfig,
        tab_config: Option<TabConfig>,
    ) -> Result<(Self, AppTask)> {
        let (name, (tab_pane_state, task), floating) = match tab_config {
            Some(TabConfig {
                name,
                pane,
                floating_pane,
            }) => (
                name,
                TabPanesState::new(terminal_config, keybinds_config, Some(pane))?,
                floating_pane
                    .map(|pane| TabPanesState::new(terminal_config, keybinds_config, Some(pane)))
                    .transpose()?,
            ),
            None => (
                None,
                TabPanesState::new(terminal_config, keybinds_config, None)?,
                None,
            ),
        };
        let mut panes = HashMap::from_iter([(TabPanesType::Normal, tab_pane_state)]);

        if let Some((tab_pane_state, _)) = floating {
            panes.insert(TabPanesType::Floating, tab_pane_state);
        }

        let tab = Tab {
            id: Uuid::now_v7(),
            name,
            panes,
            current_panes_type: TabPanesType::Normal,
        };

        Ok((tab, task))
    }

    pub fn view(&self) -> AppElement<'_> {
        let normal_panes = match self.panes.get(&TabPanesType::Normal) {
            Some(panes) => panes.view(),
            None => AppElement::from(center(Spinner::new().width(40).height(40))),
        };

        match self.current_panes_type {
            TabPanesType::Normal => normal_panes,
            TabPanesType::Floating => {
                let floating_panes =
                    Float::new(center(match self.panes.get(&TabPanesType::Floating) {
                        Some(panes) => panes.view(),
                        None => AppElement::from(Spinner::new().width(40).height(40)),
                    }));

                stack![
                    normal_panes,
                    center(floating_panes)
                        .style(|theme| {
                            let mut style = container::dark(theme);

                            if let Some(iced::Background::Color(color)) = &mut style.background {
                                color.a = 0.7;
                            }

                            style
                        })
                        .padding(15)
                ]
                .into()
            }
        }
    }

    pub fn update_pane(&mut self, pane_msg: &IdPaneMessage) -> Option<AppTask> {
        self.panes.iter_mut().find_map(|(ty, p)| {
            p.update_pane(self.id, pane_msg, matches!(ty, TabPanesType::Floating))
        })
    }

    pub fn subscription(&self) -> AppSubscription {
        AppSubscription::batch(
            self.panes
                .values()
                .flat_map(|p| p.panes.iter().map(|(_, p)| p))
                .map(PaneState::subscription),
        )
    }

    pub fn split_focused(
        &mut self,
        direction: SplitDirection,
        terminal_config: &TerminalConfig,
        keybinds_config: &KeyBindsConfig,
    ) -> Result<AppTask> {
        let Some(panes) = self.panes.get_mut(&self.current_panes_type) else {
            return Ok(AppTask::none());
        };

        panes.split_focused(direction, terminal_config, keybinds_config)
    }

    pub fn focus_pane_directional(&mut self, direction: MoveFocusDirection) -> Option<AppTask> {
        let panes = self.panes.get_mut(&self.current_panes_type)?;
        panes.focus_pane_directional(direction)
    }

    pub fn focus_pane(&mut self, id: Uuid) -> AppTask {
        self.panes
            .iter_mut()
            .find_map(|(_, p)| p.focus_pane(id))
            .unwrap_or_else(AppTask::none)
    }

    pub fn close_pane(&mut self, id: Uuid) -> AppTask {
        self.panes
            .iter_mut()
            .find_map(|(ty, p)| p.close_pane(self.id, id, matches!(ty, TabPanesType::Floating)))
            .unwrap_or_else(AppTask::none)
    }

    pub fn close_focused_pane(&mut self) -> AppTask {
        self.panes
            .get_mut(&self.current_panes_type)
            .map(|s| {
                AppTask::done(
                    IdPaneMessage {
                        id: s.focused_pane,
                        msg: PaneMessage::Close,
                    }
                    .into(),
                )
            })
            .unwrap_or_else(AppTask::none)
    }

    pub fn move_focused_pane(&mut self, direction: MoveFocusDirection) -> AppTask {
        let Some(panes) = self.panes.get_mut(&self.current_panes_type) else {
            return AppTask::none();
        };

        panes
            .move_focused_pane(direction)
            .unwrap_or_else(AppTask::none)
    }

    pub fn toggle_floating(
        &mut self,
        terminal_config: &TerminalConfig,
        keybinds_config: &KeyBindsConfig,
    ) -> AppTask {
        let focused_pane_pwd = self
            .panes
            .get(&self.current_panes_type)
            .and_then(|p| p.focused_pane())
            .map(|(_, p)| p.pwd.clone());

        self.current_panes_type = match self.current_panes_type {
            TabPanesType::Normal => TabPanesType::Floating,
            TabPanesType::Floating => TabPanesType::Normal,
        };

        let mut tasks = vec![];

        if !self.panes.contains_key(&self.current_panes_type) {
            let (tab_pane_state, task) = match TabPanesState::new(
                terminal_config,
                keybinds_config,
                focused_pane_pwd.map(|pwd| PaneConfig {
                    working_directory: Some(pwd),
                    program: None,
                    split: None,
                }),
            ) {
                Ok(res) => res,
                Err(err) => {
                    return AppTask::done(AppMsg::Error {
                        message: err.to_string(),
                        critical: false,
                    });
                }
            };
            self.panes.insert(self.current_panes_type, tab_pane_state);

            tasks.push(task);
        }

        let panes = self.panes.get_mut(&self.current_panes_type).unwrap();

        tasks.push(AppTask::done(AppMsg::FocusPane(panes.focused_pane)));

        AppTask::batch(tasks)
    }

    pub fn toggle_stacking(&mut self) {
        let Some(panes) = self.panes.get_mut(&self.current_panes_type) else {
            return;
        };

        panes.stacking = !panes.stacking;
    }

    pub fn pane(&self, id: Uuid) -> Option<(&pane_grid::Pane, &PaneState)> {
        self.panes.iter().find_map(|(_, p)| p.pane(id))
    }

    pub fn focused_pane(&self) -> Option<(&pane_grid::Pane, &PaneState)> {
        self.panes
            .get(&self.current_panes_type)
            .and_then(|p| p.focused_pane())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabPanesType {
    Normal,
    Floating,
}

#[derive(Debug)]
pub struct TabPanesState {
    pub panes: pane_grid::State<PaneState>,
    pub focused_pane: Uuid,
    pub stacking: bool,
}

impl TabPanesState {
    pub fn new(
        terminal_config: &TerminalConfig,
        keybinds_config: &KeyBindsConfig,
        root_node_config: Option<PaneConfig>,
    ) -> Result<(Self, AppTask)> {
        let root_pane_id = Uuid::now_v7();

        fn add_split(
            pane: pane_grid::Pane,
            state: &mut pane_grid::State<PaneState>,

            direction: SplitDirection,
            ratio: f32,
            PaneConfig {
                working_directory,
                program,
                split,
            }: &PaneConfig,

            terminal_config: &TerminalConfig,
            keybinds_config: &KeyBindsConfig,
        ) -> Result<()> {
            let id = Uuid::now_v7();

            let (new_pane, new_split) = state
                .split(
                    direction.into(),
                    pane,
                    PaneState::builder()
                        .id(id)
                        .terminal_config(terminal_config)
                        .keybinds_config(keybinds_config)
                        .maybe_working_directory(working_directory.clone())
                        .maybe_program_config(program.clone())
                        .build()?,
                )
                .ok_or_report()?;
            state.resize(new_split, ratio);

            if let Some(PaneSplitConfig {
                direction,
                ratio,
                child,
            }) = split
            {
                add_split(
                    new_pane,
                    state,
                    *direction,
                    (*ratio).into(),
                    child.as_ref(),
                    terminal_config,
                    keybinds_config,
                )?;
            }

            Ok(())
        }

        let task = AppTask::done(AppMsg::FocusPane(root_pane_id));
        let panes = match root_node_config {
            Some(PaneConfig {
                working_directory,
                program,
                split,
            }) => {
                let root_pane_state = PaneState::builder()
                    .id(root_pane_id)
                    .terminal_config(terminal_config)
                    .keybinds_config(keybinds_config)
                    .maybe_working_directory(working_directory)
                    .maybe_program_config(program)
                    .build()?;

                let (mut panes, root_pane) = pane_grid::State::new(root_pane_state);

                if let Some(PaneSplitConfig {
                    direction,
                    ratio,
                    child,
                }) = split
                {
                    add_split(
                        root_pane,
                        &mut panes,
                        direction,
                        ratio.into(),
                        child.as_ref(),
                        terminal_config,
                        keybinds_config,
                    )?;
                }

                panes
            }
            None => {
                let pane_state = PaneState::builder()
                    .id(root_pane_id)
                    .terminal_config(terminal_config)
                    .keybinds_config(keybinds_config)
                    .build()?;
                let (panes, _) = pane_grid::State::new(pane_state);

                panes
            }
        };

        Ok((
            TabPanesState {
                panes,
                focused_pane: root_pane_id,
                stacking: false,
            },
            task,
        ))
    }

    fn view(&self) -> AppElement<'_> {
        match self.stacking {
            true => {
                let pane = match self.pane(self.focused_pane) {
                    Some((_, state)) => state.view(self.focused_pane == state.id),
                    None => center(Spinner::new().width(40).height(40)).into(),
                };
                let pane_list = center(scrollable(
                    column(
                        self.panes
                            .iter()
                            .sorted_by_key(|(p, _)| *p)
                            .map(|(_, state)| {
                                button(space().width(10).height(10))
                                    .style(|theme, status| {
                                        let is_focused = state.id == self.focused_pane;
                                        let status = match is_focused {
                                            true => button::Status::Active,
                                            false => match status {
                                                button::Status::Hovered => button::Status::Active,
                                                _ => button::Status::Hovered,
                                            },
                                        };

                                        let mut style = button::primary(theme, status);

                                        if matches!(status, button::Status::Hovered)
                                            && let Some(background) = &mut style.background
                                        {
                                            *background = background.scale_alpha(0.8);
                                        }

                                        style
                                    })
                                    .on_press(AppMsg::FocusPane(state.id))
                                    .into()
                            }),
                    )
                    .spacing(10),
                ))
                .padding(Padding::default().right(10).top(75).bottom(75))
                .width(Length::Shrink);

                stack![pane, row![space().width(Length::Fill), pane_list]]
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .into()
            }
            false => pane_grid(&self.panes, |_pane, state, _| {
                pane_grid::Content::new(responsive(|_| state.view(state.id == self.focused_pane)))
                    .style(match self.focused_pane == state.id {
                        true => style::pane_focused,
                        false => style::pane_active,
                    })
            })
            .spacing(4)
            .into(),
        }
    }

    pub fn split_focused(
        &mut self,
        direction: SplitDirection,
        terminal_config: &TerminalConfig,
        keybinds_config: &KeyBindsConfig,
    ) -> Result<AppTask> {
        let Some((focused_pane, focused_pane_state)) = self.focused_pane() else {
            return Ok(AppTask::none());
        };

        let pane_id = Uuid::now_v7();
        let pane_state = PaneState::builder()
            .id(pane_id)
            .terminal_config(terminal_config)
            .keybinds_config(keybinds_config)
            .working_directory(focused_pane_state.pwd.clone())
            .build()?;

        self.panes.split(
            match direction {
                SplitDirection::Vertical => pane_grid::Axis::Vertical,
                SplitDirection::Horizontal => pane_grid::Axis::Horizontal,
            },
            *focused_pane,
            pane_state,
        );

        Ok(AppTask::done(AppMsg::FocusPane(pane_id)))
    }

    pub fn focus_pane_directional(&mut self, direction: MoveFocusDirection) -> Option<AppTask> {
        match self.stacking {
            true => {
                if matches!(
                    direction,
                    MoveFocusDirection::Left | MoveFocusDirection::Right
                ) {
                    return None;
                }

                let list = self
                    .panes
                    .iter()
                    .sorted_by_key(|(p, _)| *p)
                    .collect::<Vec<_>>();

                let focused_ind = list
                    .iter()
                    .enumerate()
                    .find_map(|(ind, (_, state))| (state.id == self.focused_pane).then_some(ind))?;

                Some(match direction {
                    MoveFocusDirection::Up => match focused_ind > 0 {
                        true => AppTask::done(AppMsg::FocusPane(list[focused_ind - 1].1.id)),
                        false => AppTask::none(),
                    },
                    MoveFocusDirection::Down => match focused_ind < list.len() - 1 {
                        true => AppTask::done(AppMsg::FocusPane(list[focused_ind + 1].1.id)),
                        false => AppTask::none(),
                    },
                    _ => unreachable!(),
                })
            }
            false => {
                let (focused_pane, _) = self.pane(self.focused_pane)?;

                let new_focus_pane = self
                    .panes
                    .adjacent(
                        *focused_pane,
                        match direction {
                            MoveFocusDirection::Up => pane_grid::Direction::Up,
                            MoveFocusDirection::Down => pane_grid::Direction::Down,
                            MoveFocusDirection::Left => pane_grid::Direction::Left,
                            MoveFocusDirection::Right => pane_grid::Direction::Right,
                        },
                    )
                    .and_then(|ap| {
                        self.panes
                            .iter()
                            .find_map(|(p, s)| (p == &ap).then_some(s.id))
                    })?;

                Some(AppTask::done(AppMsg::FocusPane(new_focus_pane)))
            }
        }
    }

    pub fn focus_pane(&mut self, id: Uuid) -> Option<AppTask> {
        let task = {
            let (_, p) = self.pane(id)?;
            p.focus()
        };
        self.focused_pane = id;

        Some(task)
    }

    pub fn close_pane(&mut self, tab_id: Uuid, id: Uuid, floating: bool) -> Option<AppTask> {
        let grid_id = self
            .panes
            .iter()
            .find_map(|(grid_id, p)| (p.id == id).then_some(grid_id))?;

        if self.panes.len() <= 1 {
            return Some(AppTask::done(match floating {
                true => AppMsg::TabResetFloating(tab_id),
                false => AppMsg::CloseTab(tab_id),
            }));
        }

        let (_, neighbor) = self.panes.close(*grid_id)?;

        self.panes
            .get(neighbor)
            .map(|s| AppTask::done(AppMsg::FocusPane(s.id)))
    }

    pub fn move_focused_pane(&mut self, direction: MoveFocusDirection) -> Option<AppTask> {
        let (focused, focused_state) = self.focused_pane()?;
        let focused_pane = *focused;
        let focused_id = focused_state.id;

        match self.panes.adjacent(focused_pane, direction.into()) {
            Some(adjacent) => {
                self.panes.swap(focused_pane, adjacent);
            }
            None => {
                self.panes.move_to_edge(focused_pane, direction.into());
            }
        }

        Some(AppTask::done(AppMsg::FocusPane(focused_id)))
    }

    fn update_pane(
        &mut self,
        tab_id: Uuid,
        IdPaneMessage { id, msg }: &IdPaneMessage,
        floating: bool,
    ) -> Option<AppTask> {
        let is_focused = self.focused_pane == *id;
        let (_, p) = self.pane_mut(*id)?;

        match msg {
            PaneMessage::Resize(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(*split, *ratio);
                Some(AppTask::none())
            }
            PaneMessage::Dragged(event) => match event {
                pane_grid::DragEvent::Picked { pane } => {
                    let p = self.panes.get(*pane)?;
                    Some(AppTask::done(AppMsg::FocusPane(p.id)))
                }
                pane_grid::DragEvent::Dropped { pane, target } => {
                    self.panes.drop(*pane, *target);

                    let p = self.panes.get(*pane)?;
                    Some(AppTask::done(AppMsg::FocusPane(p.id)))
                }
                pane_grid::DragEvent::Canceled { .. } => None,
            },
            PaneMessage::Close => self.close_pane(tab_id, *id, floating),
            msg => p.update(msg, is_focused),
        }
    }

    fn pane(&self, id: Uuid) -> Option<(&pane_grid::Pane, &PaneState)> {
        self.panes.iter().find(|(_, p)| p.id == id)
    }

    fn pane_mut(&mut self, id: Uuid) -> Option<(&pane_grid::Pane, &mut PaneState)> {
        self.panes.iter_mut().find(|(_, p)| p.id == id)
    }

    fn focused_pane(&self) -> Option<(&pane_grid::Pane, &PaneState)> {
        self.panes.iter().find(|(_, p)| p.id == self.focused_pane)
    }

    pub fn focused_pane_mut(&mut self) -> Option<(&pane_grid::Pane, &mut PaneState)> {
        self.panes
            .iter_mut()
            .find(|(_, p)| p.id == self.focused_pane)
    }
}

mod style {
    use iced::widget::container;
    use iced::{Border, Theme};

    pub fn pane_active(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            background: Some(palette.background.weak.color.into()),
            border: Border {
                width: 2.0,
                color: palette.background.strong.color,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn pane_focused(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            background: Some(palette.background.weak.color.into()),
            border: Border {
                width: 2.0,
                color: palette.background.strong.color,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
