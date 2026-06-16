use bon::bon;
use iced::widget::{pane_grid, responsive};
use rootcause::Result;
use uuid::Uuid;

use crate::{
    app::{AppElement, AppSubscription, AppTask},
    config::terminal::TerminalConfig,
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
    pub fn new(name: Option<String>, terminal_config: TerminalConfig) -> Result<(Self, AppTask)> {
        let root_pane_id = Uuid::now_v7();
        let pane_state = PaneState::builder()
            .id(root_pane_id)
            .terminal_config(terminal_config)
            .build()?;
        let task = pane_state.focus();

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

    pub fn pane(&self, id: Uuid) -> Option<&PaneState> {
        match self
            .panes
            .iter()
            .find_map(|(_, p)| (p.id == id).then_some(p))
        {
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

    pub fn pane_mut(&mut self, id: Uuid) -> Option<&mut PaneState> {
        self.panes
            .iter_mut()
            .find_map(|(_, p)| (p.id == id).then_some(p))
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
                .map(|s| s.focus())
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
