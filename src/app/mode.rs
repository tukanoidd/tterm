pub mod terminal;
pub mod webview;

use serde::{Deserialize, Serialize};
use strum::VariantArray;
use tterm_macros::modes;

use crate::{
    app::{AppElement, AppMsg, AppSubscription, AppTask, MainState},
    config::{
        Config,
        keybinds::{KeyBind, KeyBindsConfig},
    },
};

modes![Terminal, WebView];

pub trait TTermMode
where
    Self: Sized,
{
    type Message: TTermModeMessage<Self>;
    type KeyBindPanelType: TTermModeKeyBindPanelType;
    type Action: TTermModeAction<Self>;
    type Config: TTermModeConfig<Self>;

    type State;

    #[inline]
    fn view<'a>(self, config: &'a Config, state: &'a MainState) -> AppElement<'a>
    where
        Self: Sized + 'static,
        Config: AsRef<Self::Config>,
        MainState: AsRef<Self::State>,
    {
        self.view_impl(config.as_ref(), state.as_ref()).into()
    }

    fn view_impl<'a>(
        self,
        config: &'a Self::Config,
        state: &'a Self::State,
    ) -> impl Into<AppElement<'a>>;

    #[inline]
    fn update<'a>(
        self,
        message: Self::Message,
        config: &'a Config,
        state: &'a mut MainState,
    ) -> AppTask
    where
        Self: Sized + 'static,
        Config: AsRef<Self::Config>,
        MainState: AsMut<Self::State>,
    {
        self.update_impl(message, config.as_ref(), state.as_mut())
    }

    fn update_impl<'a>(
        self,
        message: Self::Message,
        config: &'a Self::Config,
        state: &'a mut Self::State,
    ) -> AppTask;

    #[inline]
    fn subscription<'a>(self, config: &'a Config, state: &'a MainState) -> AppSubscription
    where
        Self: Sized + 'static,
        Config: AsRef<Self::Config>,
        MainState: AsRef<Self::State>,
    {
        let conf = config.as_ref();
        let keybind_subscription = conf.keybinds().subscription(config.general.reactive_panels);

        AppSubscription::batch([
            keybind_subscription,
            self.subscription_impl(conf, state.as_ref()),
        ])
    }

    fn subscription_impl<'a>(
        self,
        config: &'a Self::Config,
        state: &'a Self::State,
    ) -> AppSubscription;
}

pub trait TTermModeMessage<M>
where
    M: TTermMode,
    Self: Into<AppMsg>,
{
    fn panel_toggle(ty: M::KeyBindPanelType, force: Option<bool>) -> Self;
}

pub trait TTermModeKeyBindPanelType
where
    Self: std::fmt::Debug
        + Clone
        + Copy
        + PartialEq
        + Eq
        + std::hash::Hash
        + VariantArray
        + Serialize
        + for<'de> Deserialize<'de>
        + Sync
        + Send
        + 'static,
{
    fn title(&self) -> String;
}

pub trait TTermModeConfig<M>
where
    M: TTermMode,
{
    fn keybinds(&self) -> &KeyBindsConfig<M>;
}

pub type ModeBindsList<M> = Vec<(
    <M as TTermMode>::KeyBindPanelType,
    Vec<(KeyBind, <M as TTermMode>::Action)>,
)>;

pub trait TTermModeAction<M>
where
    M: TTermMode,
    Self: std::fmt::Debug
        + std::fmt::Display
        + Clone
        + std::hash::Hash
        + Into<<M as TTermMode>::Message>
        + Into<AppMsg>
        + Serialize
        + for<'de> Deserialize<'de>
        + Sync
        + Send
        + 'static,
{
    fn default_keybinds() -> ModeBindsList<M>;
}
