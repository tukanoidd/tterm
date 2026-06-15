use bon::bon;
use iced::{
    Length,
    widget::{center, column, container, row, text},
};
use rootcause::Result;
use uuid::Uuid;

use crate::{
    app::{AppElement, AppSubscription, AppTask},
    config::TerminalConfig,
    multiplex::pane::Pane,
};

#[derive(Debug)]
pub struct Tab {
    pub name: Option<String>,
    root_node: TabNode,
    panes: Vec<Pane>,
}

#[bon]
impl Tab {
    #[builder]
    pub fn new(name: Option<String>, terminal_config: TerminalConfig) -> Result<(Self, AppTask)> {
        let root_pane_id = Uuid::now_v7();
        let pane = Pane::builder()
            .id(root_pane_id)
            .terminal_config(terminal_config)
            .build()?;

        let task = pane.focus();
        let tab = Tab {
            name,
            root_node: TabNode::Pane {
                id: root_pane_id,
                selected: false,
            },
            panes: vec![pane],
        };

        Ok((tab, task))
    }

    pub fn view(&self) -> AppElement<'_> {
        Self::view_node(&self.root_node, &self.panes)
    }

    fn view_node<'a>(node: &'a TabNode, panes: &'a [Pane]) -> AppElement<'a> {
        match node {
            TabNode::Pane { id, selected } => {
                let Some(pane) = panes.iter().find(|pane| &pane.id == id) else {
                    return center(text("Unknown pane!"))
                        .style(|theme| {
                            let bordered_box = container::bordered_box(theme);
                            bordered_box.border(bordered_box.border.color(theme.palette().danger))
                        })
                        .into();
                };

                container(pane.view())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |theme| {
                        let bordered_box = container::bordered_box(theme);

                        match selected {
                            true => bordered_box
                                .border(bordered_box.border.color(theme.palette().primary)),
                            false => bordered_box,
                        }
                    })
                    .into()
            }
            TabNode::Split {
                direction,
                lengths,
                nodes,
            } => {
                let widgets = nodes
                    .iter()
                    .map(|node| Self::view_node(node, panes))
                    .enumerate();

                match direction {
                    SplitDirection::Vertical => column(widgets.map(|(ind, widget)| {
                        container(widget)
                            .width(Length::Fill)
                            .height(lengths.get(ind).cloned().unwrap_or(Length::Fill))
                            .into()
                    }))
                    .into(),
                    SplitDirection::Horizontal => row(widgets.map(|(ind, widget)| {
                        container(widget)
                            .height(Length::Fill)
                            .width(lengths.get(ind).cloned().unwrap_or(Length::Fill))
                            .into()
                    }))
                    .into(),
                }
            }
        }
    }

    pub fn pane(&self, id: Uuid) -> Option<&Pane> {
        self.panes.iter().find(|p| p.id == id)
    }

    pub fn pane_mut(&mut self, id: Uuid) -> Option<&mut Pane> {
        self.panes.iter_mut().find(|p| p.id == id)
    }

    pub fn subscription(&self) -> AppSubscription {
        AppSubscription::batch(self.panes.iter().map(Pane::subscription))
    }
}

#[derive(Debug, Clone)]
pub enum TabNode {
    Pane {
        id: Uuid,
        selected: bool,
    },
    Split {
        direction: SplitDirection,
        lengths: Vec<Length>,
        nodes: Vec<TabNode>,
    },
}

#[derive(Debug, Clone)]
pub enum SplitDirection {
    Vertical,
    Horizontal,
}
