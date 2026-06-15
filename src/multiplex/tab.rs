use bon::bon;
use iced::widget::pane_grid;
use rootcause::Result;
use uuid::Uuid;

use crate::{
    app::{AppElement, AppSubscription, AppTask},
    config::TerminalConfig,
    multiplex::pane::PaneState,
};

#[derive(Debug)]
pub struct Tab {
    pub id: Uuid,
    pub name: Option<String>,

    pub panes: pane_grid::State<PaneState>,
    pub focused_pane: Uuid,
    root_pane: pane_grid::Pane,
    panes_created: usize,
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
            root_pane,
            focused_pane: root_pane_id,
            panes_created: 1,
        };

        Ok((tab, task))
    }

    pub fn view(&self) -> AppElement<'_> {
        pane_grid(&self.panes, |_pane, state, _| {
            pane_grid::Content::new(state.view())
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

        tracing::debug!("Closing Pane: {id}");

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
