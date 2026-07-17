use chumsky::prelude::*;
use derive_more::Display;
use iced::{Event, event, keyboard, widget::pane_grid};
use itertools::Itertools;
use serde::{Deserialize, Serialize, de::Visitor};

use crate::app::{
    AppMsg, AppSubscription,
    mode::{ModeBindsList, TTermMode, TTermModeAction, TTermModeMessage},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindsConfig<M>
where
    M: TTermMode,
{
    pub actions: ModeBindsList<M>,
}

impl<M> KeyBindsConfig<M>
where
    M: TTermMode,
{
    pub fn subscription(&self, reactive_panels: bool) -> AppSubscription {
        event::listen_with(move |event, _, _window_id| Some(event))
            .with((self.actions.clone(), reactive_panels))
            .map(|((actions, reactive_panels), event)| match event {
                Event::Keyboard(keyboard_event) => match keyboard_event {
                    keyboard::Event::KeyPressed {
                        key,
                        modified_key,
                        physical_key,
                        location,
                        modifiers,
                        text,
                        repeat,
                    } => vec![
                        (!repeat)
                            .then(|| {
                                actions.into_iter().find_map(|(_, binds)| {
                                    binds.into_iter().find_map(
                                        |(
                                            KeyBind {
                                                key: bind_key,
                                                modifiers: bind_modifiers,
                                            },
                                            action,
                                        )| {
                                            let iced_key: iced::keyboard::Key = bind_key.into();
                                            let iced_modifiers = bind_modifiers.into_iter().fold(
                                                keyboard::Modifiers::empty(),
                                                |mods, mod_| match mod_ {
                                                    Modifier::Ctrl => {
                                                        mods | keyboard::Modifiers::CTRL
                                                    }
                                                    Modifier::Shift => {
                                                        mods | keyboard::Modifiers::SHIFT
                                                    }
                                                    Modifier::Alt => {
                                                        mods | keyboard::Modifiers::ALT
                                                    }
                                                },
                                            );

                                            ([&key, &modified_key].contains(&&iced_key)
                                                && iced_modifiers == modifiers)
                                                .then_some(Into::<AppMsg>::into(action))
                                        },
                                    )
                                })
                            })
                            .flatten()
                            .unwrap_or_else(|| {
                                AppMsg::IcedEvent(iced::Event::Keyboard(
                                    iced::keyboard::Event::KeyPressed {
                                        key,
                                        modified_key,
                                        physical_key,
                                        location,
                                        modifiers,
                                        text,
                                        repeat,
                                    },
                                ))
                            }),
                    ],
                    keyboard::Event::ModifiersChanged(modifiers) => match reactive_panels {
                        true => {
                            let changed_mods = modifiers
                                .iter()
                                .filter_map(|m| match m {
                                    keyboard::Modifiers::SHIFT => Some(Modifier::Shift),
                                    keyboard::Modifiers::CTRL => Some(Modifier::Ctrl),
                                    keyboard::Modifiers::ALT => Some(Modifier::Alt),
                                    _ => None,
                                })
                                .collect::<Vec<_>>();

                            actions
                                .into_iter()
                                .map(|(ty, binds)| {
                                    let open =
                                        binds.into_iter().map(|(b, _)| b).unique().any(|b| {
                                            b.modifiers.iter().any(|m| changed_mods.contains(m))
                                        });

                                    <M::Message as TTermModeMessage<M>>::panel_toggle(
                                        ty,
                                        Some(open),
                                    )
                                    .into()
                                })
                                .collect::<Vec<_>>()
                        }
                        false => vec![],
                    },
                    ev => vec![AppMsg::IcedEvent(iced::Event::Keyboard(ev))],
                },
                _ => vec![AppMsg::IcedEvent(event)],
            })
            .map(AppMsg::Multiple)
    }
}

impl<M> Default for KeyBindsConfig<M>
where
    M: TTermMode,
{
    fn default() -> Self {
        Self {
            actions: <<M as TTermMode>::Action as TTermModeAction<M>>::default_keybinds(),
        }
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoveFocusDirection {
    Up,
    Down,
    Left,
    Right,
}

impl From<MoveFocusDirection> for pane_grid::Direction {
    fn from(value: MoveFocusDirection) -> Self {
        match value {
            MoveFocusDirection::Up => pane_grid::Direction::Up,
            MoveFocusDirection::Down => pane_grid::Direction::Down,
            MoveFocusDirection::Left => pane_grid::Direction::Left,
            MoveFocusDirection::Right => pane_grid::Direction::Right,
        }
    }
}

impl From<MoveFocusDirection> for pane_grid::Edge {
    fn from(value: MoveFocusDirection) -> Self {
        match value {
            MoveFocusDirection::Up => pane_grid::Edge::Top,
            MoveFocusDirection::Down => pane_grid::Edge::Bottom,
            MoveFocusDirection::Left => pane_grid::Edge::Left,
            MoveFocusDirection::Right => pane_grid::Edge::Right,
        }
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[display(
    "{}{key}",
    format!(
        "{}+",
        modifiers
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join("+")
    )
)]
pub struct KeyBind {
    pub key: Key,
    pub modifiers: Vec<Modifier>,
}

impl KeyBind {
    pub fn new(key: impl Into<Key>, mods: impl IntoIterator<Item = Modifier>) -> Self {
        Self {
            key: key.into(),
            modifiers: mods.into_iter().collect(),
        }
    }
}

impl Serialize for KeyBind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub struct KeyBindVisitor;

impl<'de> Visitor<'de> for KeyBindVisitor {
    type Value = KeyBind;

    fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            fmt,
            "A (<MOD>+)+<KEY> atring (Ctrl+Shift+N or Alt+P as examples)"
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let modifiers = Modifier::parser()
            .separated_by(just('+'))
            .collect::<Vec<_>>()
            .then_ignore(just('+'));
        let key = Key::parser();

        let res = modifiers
            .then(key)
            .map(|(modifiers, key)| KeyBind { modifiers, key })
            .parse(v)
            .into_result()
            .map_err(|errs| {
                E::custom(errs.into_iter().fold(
                    String::new(),
                    |output, err| match output.is_empty() {
                        true => err.to_string(),
                        false => format!("{output}\n{err}"),
                    },
                ))
            })?;

        Ok(res)
    }
}

impl<'de> Deserialize<'de> for KeyBind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(KeyBindVisitor)
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Named(NamedKey),
    #[display("{}", _0.to_uppercase())]
    Character(String),
}

impl Key {
    fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        NamedKey::parser().map(Self::Named).or(any()
            .repeated()
            .at_least(1)
            .to_slice()
            .map(|s: &str| Self::Character(s.to_lowercase())))
    }
}

impl From<Key> for iced::keyboard::Key {
    fn from(key: Key) -> iced::keyboard::Key {
        match key {
            Key::Named(named) => iced::keyboard::Key::Named(named.into()),
            Key::Character(char) => iced::keyboard::Key::Character(char.to_lowercase().into()),
        }
    }
}

impl From<NamedKey> for Key {
    fn from(value: NamedKey) -> Self {
        Self::Named(value)
    }
}

impl<S> From<S> for Key
where
    S: Into<String>,
{
    fn from(value: S) -> Self {
        Self::Character(value.into())
    }
}

macro_rules! named_key {
    (
        $first_name:ident $({
            alias = [$($first_alias:literal),+ $(,)?]
        })?,
        $($name:ident $({
            alias = [$($alias:literal),+ $(,)?]
        })?),+
        $(,)?
    ) => {
        #[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum NamedKey {
            $first_name,
            $($name),+
        }

        impl NamedKey {
            fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> {
                (
                    named_key!(@parse $first_name $({
                        alias = [$($first_alias),+]
                    })?)
                )
                $(
                    .or(
                        named_key!(@parse $name $({
                            alias = [$($alias),+]
                        })?)
                    ).boxed()
                )+
                    .boxed()
            }
        }

        impl From<NamedKey> for iced::keyboard::key::Named {
            fn from(value: NamedKey) -> Self {
                match value {
                    NamedKey::$first_name => Self::$first_name,
                    $(NamedKey::$name => Self::$name),+
                }
            }
        }

        pub struct NamedKeyVisitor;

        impl<'de> serde::de::Visitor<'de> for NamedKeyVisitor {
            type Value = NamedKey;

            fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(fmt, "a string denoting a named key")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                NamedKey::parser()
                    .parse(s)
                    .into_result()
                    .map_err(|errs|
                        E::custom(
                            errs
                                .into_iter()
                                .fold(
                                    String::new(),
                                    |output, err| {
                                        match output.is_empty() {
                                            true => err.to_string(),
                                            false => format!("{output}/n{err}")
                                        }

                                    }
                                )
                        )
                    )
            }
        }

        impl serde::Serialize for NamedKey {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> serde::Deserialize<'de> for NamedKey {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: serde::de::Deserializer<'de> {
                    deserializer.deserialize_str(NamedKeyVisitor)
                }
        }
    };

    (@parse $name:ident) => {
        just(stringify!($name)).to(Self::$name)
    };
    (@parse $name:ident {
        alias = [$($alias:literal),+]
    }) => {
        choice((
            just(stringify!($name)),
            $(just($alias)),+
        ))
            .to(Self::$name)
    };
}

named_key![
    Alt { alias = ["Option"] },
    AltGraph { alias = ["AltGr"] },
    CapsLock { alias = ["Caps"] },
    Control { alias = ["Ctrl"] },
    Fn,
    FnLock { alias = ["F-Lock"] },
    NumLock,
    ScrollLock { alias = ["ScrlLock"] },
    Shift,
    Symbol,
    SymbolLock,
    Meta,
    Hyper,
    Super { alias = ["Logo", "Mod", "Command", "Cmd", "Meta"] },
    Enter,
    Tab,
    Space { alias = ["Spc"] },
    ArrowDown { alias = ["Down"] },
    ArrowLeft { alias = ["Left"] },
    ArrowRight { alias = ["Right"] },
    ArrowUp { alias = ["Up"] },
    End,
    Home,
    PageDown { alias = ["PgDown"] },
    PageUp { alias = ["PgUp"] },
    Backspace { alias = ["Bckspc"] },
    Clear { alias = ["Clr"] },
    Copy,
    CrSel,
    Cut,
    Delete { alias = ["Del"] },
    EraseEof,
    ExSel,
    Insert { alias = ["Ins"] },
    Paste,
    Redo,
    Undo,
    Accept,
    Again,
    Attn,
    Cancel,
    ContextMenu,
    Escape { alias = ["Esc"] },
    Execute { alias = ["Exec"] },
    Find,
    Help,
    Pause,
    Play,
    Props,
    Select { alias = ["Sel"] },
    ZoomIn,
    ZoomOut,
    BrightnessDown { alias = ["BrDown"] },
    BrightnessUp { alias = ["BrUp"] },
    Eject,
    LogOff,
    Power { alias = ["Pwr"] },
    PowerOff { alias = ["PwrOff"] },
    PrintScreen { alias = ["PrntScrn"] },
    Hibernate,
    Standby { alias = ["Suspend", "Sleep"] },
    WakeUp,
    AllCandidates,
    Alphanumeric,
    CodeInput,
    Compose { alias = ["Multi"] },
    Convert,
    FinalMode { alias = ["Final"] },
    GroupFirst { alias = ["GrpFst", "GroupFst", "GrpFirst"] },
    GroupLast { alias = ["GrpLst", "GroupLst", "GrpLast"] },
    GroupNext { alias = ["GrpNxt", "GroupNxt", "GrpNext"] },
    GroupPrevious { alias = ["GrpPrv", "GroupPrv", "GrpPrevious", "GrpPrev", "GroupPrev"] },
    ModeChange,
    NextCandidate { alias = ["NxtCandidate"] },
    NonConvert,
    PreviousCandidate { alias = ["PrvCandidate", "PrevCandidate"] },
    Process { alias = ["Prcs", "Proc"] },
    SingleCandidate { alias = ["SngCandidate"] },
    HangulMode,
    HanjaMode,
    JunjaMode,
    Eisu,
    Hankaku,
    Hiragana,
    HiraganaKatakana,
    KanaMode,
    KanjiMode,
    Katakana,
    Romaji,
    Zenkaku,
    ZenkakuHankaku,
    Soft1,
    Soft2,
    Soft3,
    Soft4,
    ChannelDown,
    ChannelUp,
    Close,
    MailForward,
    MailReply,
    MailSend,
    MediaClose,
    MediaFastForward,
    MediaPause,
    MediaPlay,
    MediaPlayPause,
    MediaRecord,
    MediaRewind,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,
    New,
    Open { alias = ["Opn"] },
    Print { alias = ["Prnt"]  },
    Save,
    SpellCheck { alias = ["SpellChk"] },
    Key11,
    Key12,
    AudioBalanceLeft,
    AudioBalanceRight,
    AudioBassBoostDown,
    AudioBassBoostToggle,
    AudioBassBoostUp,
    AudioFaderFront,
    AudioFaderRear,
    AudioSurroundModeNext,
    AudioTrebleDown,
    AudioTrebleUp,
    AudioVolumeDown,
    AudioVolumeUp,
    AudioVolumeMute,
    MicrophoneToggle,
    MicrophoneVolumeDown,
    MicrophoneVolumeUp,
    MicrophoneVolumeMute,
    SpeechCorrectionList,
    SpeechInputToggle,
    LaunchApplication1,
    LaunchApplication2,
    LaunchCalendar,
    LaunchContacts,
    LaunchMail,
    LaunchMediaPlayer,
    LaunchMusicPlayer,
    LaunchPhone,
    LaunchScreenSaver,
    LaunchSpreadsheet,
    LaunchWebBrowser,
    LaunchWebCam,
    LaunchWordProcessor,
    BrowserBack,
    BrowserFavorites,
    BrowserForward,
    BrowserHome,
    BrowserRefresh,
    BrowserSearch,
    BrowserStop,
    AppSwitch,
    Call,
    Camera { alias = ["Cam"] },
    CameraFocus { alias = ["CamFocus"] },
    EndCall,
    GoBack,
    GoHome,
    HeadsetHook,
    LastNumberRedial,
    Notification,
    MannerMode,
    VoiceDial,
    TV,
    TV3DMode,
    TVAntennaCable,
    TVAudioDescription,
    TVAudioDescriptionMixDown,
    TVAudioDescriptionMixUp,
    TVContentsMenu,
    TVDataService,
    TVInput,
    TVInputComponent1,
    TVInputComponent2,
    TVInputComposite1,
    TVInputComposite2,
    TVInputHDMI1,
    TVInputHDMI2,
    TVInputHDMI3,
    TVInputHDMI4,
    TVInputVGA1,
    TVMediaContext,
    TVNetwork,
    TVNumberEntry,
    TVPower,
    TVRadioService,
    TVSatellite,
    TVSatelliteBS,
    TVSatelliteCS,
    TVSatelliteToggle,
    TVTerrestrialAnalog,
    TVTerrestrialDigital,
    TVTimer,
    AVRInput,
    AVRPower,
    ColorF0Red,
    ColorF1Green,
    ColorF2Yellow,
    ColorF3Blue,
    ColorF4Grey,
    ColorF5Brown,
    ClosedCaptionToggle,
    Dimmer,
    DisplaySwap,
    DVR,
    Exit,
    FavoriteClear0,
    FavoriteClear1,
    FavoriteClear2,
    FavoriteClear3,
    FavoriteRecall0,
    FavoriteRecall1,
    FavoriteRecall2,
    FavoriteRecall3,
    FavoriteStore0,
    FavoriteStore1,
    FavoriteStore2,
    FavoriteStore3,
    Guide,
    GuideNextDay,
    GuidePreviousDay,
    Info,
    InstantReplay,
    Link,
    ListProgram,
    LiveContent,
    Lock,
    MediaApps,
    MediaAudioTrack,
    MediaLast,
    MediaSkipBackward,
    MediaSkipForward,
    MediaStepBackward,
    MediaStepForward,
    MediaTopMenu,
    NavigateIn,
    NavigateNext,
    NavigateOut,
    NavigatePrevious,
    NextFavoriteChannel,
    NextUserProfile,
    OnDemand,
    Pairing,
    PinPDown,
    PinPMove,
    PinPToggle,
    PinPUp,
    PlaySpeedDown,
    PlaySpeedReset,
    PlaySpeedUp,
    RandomToggle,
    RcLowBattery,
    RecordSpeedNext,
    RfBypass,
    ScanChannelsToggle,
    ScreenModeNext,
    Settings,
    SplitScreenToggle,
    STBInput,
    STBPower,
    Subtitle,
    Teletext,
    VideoModeNext,
    Wink,
    ZoomToggle,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
];

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
}

impl Modifier {
    fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        choice((
            just("Control").or(just("Ctrl")).to(Self::Ctrl),
            just("Shift").to(Self::Shift),
            just("Alt").to(Self::Alt),
        ))
    }
}
