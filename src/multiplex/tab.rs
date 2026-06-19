use bon::bon;
use iced::widget::{pane_grid, responsive};
use rootcause::Result;
use uuid::Uuid;

use crate::{
    app::{AppElement, AppMsg, AppSubscription, AppTask},
    config::{
        keybinds::{FocusDirection, SplitDirection},
        terminal::TerminalConfig,
    },
    multiplex::pane::PaneState,
};

#[derive(Debug)]
pub struct Tab {
    pub id: Uuid,
    pub name: Option<String>,

    pub panes: pane_grid::State<PaneState>,
    pub focused_pane: Uuid,
    _root_pane: pane_grid::Pane,
}

#[bon]
impl Tab {
    #[builder]
    pub fn new(name: Option<String>, terminal_config: &TerminalConfig) -> Result<(Self, AppTask)> {
        let root_pane_id = Uuid::now_v7();
        let pane_state = PaneState::builder()
            .id(root_pane_id)
            .terminal_config(terminal_config)
            .build()?;
        let task = AppTask::done(AppMsg::FocusPane(root_pane_id));

        let (panes, root_pane) = pane_grid::State::new(pane_state);

        let tab = Tab {
            id: Uuid::now_v7(),
            name,
            panes,
            focused_pane: root_pane_id,
            _root_pane: root_pane,
        };

        Ok((tab, task))
    }

    pub fn view(&self) -> AppElement<'_> {
        pane_grid(&self.panes, |_pane, state, _| {
            pane_grid::Content::new(responsive(|_| state.view())).style(
                match self.focused_pane == state.id {
                    true => style::pane_focused,
                    false => style::pane_active,
                },
            )
        })
        .into()
    }

    pub fn subscription(&self) -> AppSubscription {
        AppSubscription::batch(
            self.panes
                .iter()
                .map(|(_, p)| p)
                .map(PaneState::subscription),
        )
    }

    pub fn split_focused(
        &mut self,
        direction: SplitDirection,
        terminal_config: &TerminalConfig,
    ) -> Result<AppTask> {
        let Some((focused_pane, _)) = self.pane(self.focused_pane) else {
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

    pub fn focus_pane(&mut self, direction: FocusDirection) -> AppTask {
        let Some((focused_pane, _)) = self.pane(self.focused_pane) else {
            return AppTask::none();
        };

        let Some(new_focus_pane) = self
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
            })
        else {
            return AppTask::none();
        };

        AppTask::done(AppMsg::FocusPane(new_focus_pane))
    }

    pub fn pane(&self, id: Uuid) -> Option<(&pane_grid::Pane, &PaneState)> {
        match self.panes.iter().find(|(_, p)| p.id == id) {
            Some(p) => Some(p),
            None => {
                tracing::trace!(
                    "Failed to find pane {id}: {:?}",
                    self.panes
                        .iter()
                        .map(|(_, PaneState { id, .. })| format!("id: {id}"))
                        .collect::<Vec<_>>()
                );
                None
            }
        }
    }

    pub fn pane_mut(&mut self, id: Uuid) -> Option<(&pane_grid::Pane, &mut PaneState)> {
        self.panes.iter_mut().find(|(_, p)| p.id == id)
    }

    pub fn close_pane(&mut self, id: Uuid) -> AppTask {
        let Some(grid_id) = self
            .panes
            .iter()
            .find_map(|(grid_id, p)| (p.id == id).then_some(grid_id))
        else {
            return AppTask::none();
        };

        if self.panes.len() <= 1 {
            return AppTask::done(crate::app::AppMsg::CloseTab(self.id));
        }

        match self.panes.close(*grid_id) {
            Some((_, neighbor)) => self
                .panes
                .get(neighbor)
                .map(|s| AppTask::done(AppMsg::FocusPane(s.id)))
                .unwrap_or_else(AppTask::none),
            None => AppTask::none(),
        }
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
