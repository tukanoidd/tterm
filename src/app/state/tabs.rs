use iced::widget::{center, pane_grid::Pane};
use iced_aw::Spinner;
use uuid::Uuid;

use crate::{
    app::{
        AppElement, AppTask,
        mode::{
            TTermMode,
            terminal::{TerminalMode, TerminalModeTabAction},
        },
    },
    multiplex::{
        pane::{IdPaneMessage, PaneState},
        tab::{Tab, TabPanesType},
    },
};

#[derive(Default)]
pub struct TabsState {
    pub list: Vec<Tab>,
    pub current: usize,

    pub rename_mode: bool,
    pub rename_content: String,
}

impl TabsState {
    pub fn view(&self) -> AppElement<'_> {
        match self.list.get(self.current) {
            None => center(Spinner::new().width(20).height(20)).into(),
            Some(tab) => tab.view(),
        }
    }

    pub fn current_tab(&self) -> Option<&Tab> {
        self.list.get(self.current)
    }

    pub fn current_tab_mut(&mut self) -> Option<&mut Tab> {
        self.list.get_mut(self.current)
    }

    pub fn focused_pane(&self) -> Option<(&Pane, &PaneState)> {
        self.list.get(self.current).and_then(|t| t.focused_pane())
    }

    pub fn rename_current_tab(&mut self, new_name: impl Into<String>) -> AppTask {
        let Some(tab) = self.current_tab_mut() else {
            return AppTask::none();
        };

        tab.name = Some(new_name.into());
        self.rename_mode = false;

        AppTask::done(TerminalModeTabAction::Select(self.current).into())
    }

    pub fn rename_input(&mut self, new_input: impl Into<String>) {
        self.rename_content = new_input.into();
    }

    pub fn tab_mut(&mut self, id: Uuid) -> Option<&mut Tab> {
        self.list.iter_mut().find(|t| t.id == id)
    }

    pub fn close(&mut self, id: Uuid) -> AppTask {
        let Some(tab) = self
            .list
            .iter()
            .enumerate()
            .find_map(|(ind, tab)| (tab.id == id).then_some(ind))
        else {
            return AppTask::none();
        };

        self.list.remove(tab);
        self.current = tab.saturating_sub(1);

        if self.list.is_empty() {
            return iced::exit();
        }

        AppTask::done(
            TerminalModeTabAction::Select(tab.saturating_sub(1.clamp(0, self.list.len()))).into(),
        )
    }

    pub fn reset_floating(&mut self, id: Uuid) -> AppTask {
        let Some(tab) = self.tab_mut(id) else {
            return AppTask::none();
        };

        tab.panes.remove(&TabPanesType::Floating);

        AppTask::done(TerminalModeTabAction::FocusedToggleFloating.into())
    }

    pub fn focus_pane(&mut self, pane_id: Uuid) -> AppTask {
        let Some(tab) = self.current_tab_mut() else {
            return AppTask::none();
        };

        tab.focus_pane(pane_id).chain(AppTask::done(
            <TerminalMode as TTermMode>::Message::UpdateFocusedDirectoryTree.into(),
        ))
    }

    pub fn update_pane(&mut self, pane_msg: IdPaneMessage) -> AppTask {
        self.list
            .iter_mut()
            .find_map(|t| t.update_pane(&pane_msg))
            .unwrap_or_else(AppTask::none)
    }
}
