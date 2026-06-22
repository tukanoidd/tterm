use derive_more::Display;
use iced::widget::pane_grid;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SplitDirection {
    Vertical,
    #[default]
    Horizontal,
}

impl From<SplitDirection> for pane_grid::Axis {
    fn from(value: SplitDirection) -> Self {
        match value {
            SplitDirection::Vertical => pane_grid::Axis::Vertical,
            SplitDirection::Horizontal => pane_grid::Axis::Horizontal,
        }
    }
}
