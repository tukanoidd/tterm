use std::collections::HashMap;

use bon::bon;
use iced::{
    Length,
    widget::{Float, button, center, column, container, pane_grid, responsive, row, space, stack},
};
use iced_aw::Spinner;
use itertools::Itertools;
use rootcause::Result;
use uuid::Uuid;

use crate::{
    app::{AppElement, AppMsg, AppSubscription, AppTask},
    config::{
        keybinds::{FocusDirection, SplitDirection},
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
    pub fn new(name: Option<String>, terminal_config: &TerminalConfig) -> Result<(Self, AppTask)> {
        let (tab_pane_state, task) = TabPanesState::new(terminal_config)?;
        let panes = HashMap::from_iter([(TabPanesType::Normal, tab_pane_state)]);

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
    ) -> Result<AppTask> {
        let Some(panes) = self.panes.get_mut(&self.current_panes_type) else {
            return Ok(AppTask::none());
        };

        panes.split_focused(direction, terminal_config)
    }

    pub fn focus_pane_directional(&mut self, direction: FocusDirection) -> Option<AppTask> {
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

    pub fn toggle_floating(&mut self, terminal_config: &TerminalConfig) -> AppTask {
        self.current_panes_type = match self.current_panes_type {
            TabPanesType::Normal => TabPanesType::Floating,
            TabPanesType::Floating => TabPanesType::Normal,
        };

        let mut tasks = vec![];

        if !self.panes.contains_key(&self.current_panes_type) {
            let (tab_pane_state, task) = match TabPanesState::new(terminal_config) {
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
    pub root_pane: pane_grid::Pane,
    pub stacking: bool,
}

impl TabPanesState {
    pub fn new(terminal_config: &TerminalConfig) -> Result<(Self, AppTask)> {
        let root_pane_id = Uuid::now_v7();
        let pane_state = PaneState::builder()
            .id(root_pane_id)
            .terminal_config(terminal_config)
            .build()?;
        let task = AppTask::done(AppMsg::FocusPane(root_pane_id));

        let (panes, root_pane) = pane_grid::State::new(pane_state);

        Ok((
            TabPanesState {
                panes,
                focused_pane: root_pane_id,
                root_pane,
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
                let pane_list = center(
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
                )
                .padding(5)
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
    ) -> Result<AppTask> {
        let Some((focused_pane, _)) = self.focused_pane() else {
            return Ok(AppTask::none());
        };

        let pane_id = Uuid::now_v7();
        let pane_state = PaneState::builder()
            .id(pane_id)
            .terminal_config(terminal_config)
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

    pub fn focus_pane_directional(&mut self, direction: FocusDirection) -> Option<AppTask> {
        match self.stacking {
            true => {
                if matches!(direction, FocusDirection::Left | FocusDirection::Right) {
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
                    FocusDirection::Up => match focused_ind > 0 {
                        true => AppTask::done(AppMsg::FocusPane(list[focused_ind - 1].1.id)),
                        false => AppTask::none(),
                    },
                    FocusDirection::Down => match focused_ind < list.len() - 1 {
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
                            FocusDirection::Up => pane_grid::Direction::Up,
                            FocusDirection::Down => pane_grid::Direction::Down,
                            FocusDirection::Left => pane_grid::Direction::Left,
                            FocusDirection::Right => pane_grid::Direction::Right,
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

    fn update_pane(
        &mut self,
        tab_id: Uuid,
        IdPaneMessage { id, msg }: &IdPaneMessage,
        floating: bool,
    ) -> Option<AppTask> {
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
            msg => p.update(msg),
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
