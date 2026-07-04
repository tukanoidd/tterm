use std::path::PathBuf;

use iced::{Length, widget::container};
use iced_swdir_tree::{DirectoryTree, DirectoryTreeEvent};

use crate::{
    app::{AppElement, AppMsg, AppTask},
    multiplex::pane::PaneState,
};

pub struct DirectoryTreeState {
    pub show: bool,
    pub directory_tree: DirectoryTree,
}

impl DirectoryTreeState {
    pub fn new(home_dir: impl Into<PathBuf>) -> Self {
        let directory_tree = DirectoryTree::new(home_dir.into()).with_prefetch_limit(1);

        Self {
            show: false,
            directory_tree,
        }
    }

    pub fn view(&self) -> Option<AppElement<'_>> {
        self.show.then(|| {
            container(self.directory_tree.view(AppMsg::DirectoryTree))
                .width(Length::Fixed(400.0))
                .into()
        })
    }

    pub fn update(&mut self, event: DirectoryTreeEvent) -> AppTask {
        self.directory_tree.update(event).map(AppMsg::DirectoryTree)
    }

    pub fn update_path<'a>(&'a mut self, focused_pane: &'a PaneState) -> AppTask {
        self.directory_tree = DirectoryTree::new(focused_pane.pwd.clone()).with_prefetch_limit(1);
        AppTask::done(DirectoryTreeEvent::Toggled(focused_pane.pwd.clone()).into())
    }
}
