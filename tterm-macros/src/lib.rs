use std::{ffi::OsStr, fs::DirEntry, path::Path};

use chumsky::prelude::Rich;
use convert_case::ccase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, format_ident, quote};
use syn::{Expr, Ident, LitStr, Token, Variant, bracketed, parse::Parse, parse_macro_input};

#[proc_macro]
pub fn actions(input: TokenStream) -> TokenStream {
    let Actions { types } = parse_macro_input!(input);

    let tterm_action_enum = {
        let variants = types
            .iter()
            .map(|ActionType { name, .. }| {
                let type_name = format_ident!("TTerm{name}Action");
                quote!(#name(#type_name))
            })
            .collect::<Vec<_>>();
        let keybind_panel_type_enum = {
            let variants = types
                .iter()
                .map(|ActionType { name, .. }| name)
                .collect::<Vec<_>>();
            let title_match_arms = types.iter().map(|ActionType { name, .. }| quote!(Self::#name => format!("{} Actions", stringify!(#name))));

            quote! {
                #[derive(
                    Debug, derive_more::Display,
                    Clone, Copy,
                    PartialEq, Eq,
                    Hash,
                    strum::VariantArray,
                    serde::Serialize, serde::Deserialize,
                )]
                pub enum KeyBindPanelType {
                    #(#variants),*
                }

                impl KeyBindPanelType {
                    pub fn title(&self) -> String {
                        match self {
                            #(#title_match_arms),*
                        }
                    }
                }

                impl<'a> From<&'a TTermAction> for KeyBindPanelType {
                    fn from(value: &'a TTermAction) -> Self {
                        match value {
                            #(TTermAction::#variants(_) => Self::#variants),*
                        }
                    }
                }
            }
        };

        let impls = {
            let default_keybinds = {
                let binds = types.iter().map(|ActionType { name, .. }| {
                    let type_name = format_ident!("TTerm{name}Action");

                    quote!((
                        KeyBindPanelType::#name,
                        Vec::from_iter(
                            #type_name::default_keybinds()
                                .into_iter()
                                .map(|(bind, action)| (
                                    bind,
                                    Self::#name(action)
                                ))
                        )
                    ))
                });

                quote! {
                    fn default_keybinds() -> Vec<(KeyBindPanelType, Vec<(KeyBind, TTermAction)>)> {
                        Vec::from_iter([#(#binds),*])
                    }
                }
            };

            let from_trait = {
                quote! {
                    impl<IA> From<IA> for crate::app::AppMsg where IA: Into<TTermAction> {
                        fn from(value: IA) -> crate::app::AppMsg {
                            crate::app::AppMsg::Action(value.into())
                        }
                    }
                }
            };

            quote! {
                impl TTermAction {
                    #default_keybinds
                }

                #from_trait
            }
        };

        quote! {
            #[derive(
                Debug, derive_more::Display,
                Clone, Hash,
                derive_more::From,
                serde::Serialize, serde::Deserialize
            )]
            pub enum TTermAction {
                #(#variants),*
            }

            #impls

            #keybind_panel_type_enum
        }
    };

    let types = types.iter().map(|ActionType { name, actions }| {
        let type_name = format_ident!("TTerm{name}Action");
        let variants = actions.iter().map(|Action { variant, .. }| variant);

        let impl_methods = {
            let default_keybinds = {
                let binds = actions.iter().flat_map(
                    |Action {
                         variant: Variant { ident, .. },
                         default_bindings,
                     }| {
                        let ident = ident.clone();

                        default_bindings.iter().map(
                            move |KeyBind {
                                      modifiers,
                                      key,
                                      as_action,
                                  }| {
                                let key = match key {
                                    Key::Named(ident) => quote!(Key::Named(NamedKey::#ident)),
                                    Key::Character(lit_str) => {
                                        quote!(Key::Character(#lit_str.into()))
                                    }
                                };
                                let modifiers = modifiers.iter().map(|m| quote!(Modifier::#m));

                                quote! {(
                                    KeyBind::new(#key, [#(#modifiers),*]),
                                    Self::#ident #as_action
                                )}
                            },
                        )
                    },
                );

                quote! {
                    pub fn default_keybinds() -> impl IntoIterator<Item = (KeyBind, #type_name)> {
                        [#(#binds),*]
                    }
                }
            };

            quote! {
                impl #type_name {
                    #default_keybinds
                }
            }
        };

        quote! {
            #[derive(
                Debug, derive_more::Display,
                Clone, Hash,
                serde::Serialize, serde::Deserialize
            )]
            pub enum #type_name {
                #(#variants),*
            }

            #impl_methods
        }
    });

    quote! {
        #tterm_action_enum

        #(#types)*
    }
    .into()
}

struct Actions {
    types: Vec<ActionType>,
}

impl Parse for Actions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let types = input
            .parse_terminated(ActionType::parse, Token![,])?
            .into_iter()
            .collect::<Vec<_>>();

        Ok(Self { types })
    }
}

struct ActionType {
    name: Ident,
    actions: Vec<Action>,
}

impl Parse for ActionType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;

        let actions_input;
        let _br = bracketed!(actions_input in input);
        let actions = actions_input
            .parse_terminated(Action::parse, Token![,])?
            .into_iter()
            .collect::<Vec<_>>();

        Ok(Self { name, actions })
    }
}

struct Action {
    variant: Variant,
    default_bindings: Vec<KeyBind>,
}

impl Parse for Action {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let variant: Variant = input.parse()?;

        let default_bindings_input;
        let _br = bracketed!(default_bindings_input in input);
        let default_bindings = default_bindings_input
            .parse_terminated(KeyBind::parse, Token![,])?
            .into_iter()
            .collect();

        Ok(Self {
            variant,
            default_bindings,
        })
    }
}

struct KeyBind {
    modifiers: Vec<Ident>,
    key: Key,
    as_action: Option<Expr>,
}

impl Parse for KeyBind {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _at: Token![@] = input.parse()?;
        let mod_input;
        let _br = bracketed!(mod_input in input);
        let modifiers = mod_input
            .parse_terminated(Ident::parse, Token![+])?
            .into_iter()
            .collect::<Vec<_>>();

        let _plus: Token![+] = input.parse()?;
        let key: Key = input.parse()?;

        let as_action = match input.peek(Token![=>]) {
            true => {
                let _arrow: Token![=>] = input.parse()?;
                Some(input.parse()?)
            }
            false => None,
        };

        Ok(Self {
            modifiers,
            key,
            as_action,
        })
    }
}

enum Key {
    Named(Ident),
    Character(LitStr),
}

impl Parse for Key {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        match input.peek(Token![@]) {
            true => {
                let _at: Token![@] = input.parse()?;
                let named: Ident = input.parse()?;

                Ok(Self::Named(named))
            }
            false => {
                let char: LitStr = input.parse()?;
                Ok(Self::Character(char))
            }
        }
    }
}

#[proc_macro]
pub fn fonts(input: TokenStream) -> TokenStream {
    let fonts_dir = parse_macro_input!(input as LitStr);
    let span = fonts_dir.span();
    let fonts_dir_str = fonts_dir.value();

    if !Path::new(&fonts_dir_str).exists() {
        panic!("{fonts_dir_str:?} doesn't exist!");
    }

    let fonts = get_dir_entries(fonts_dir_str)
        .unwrap()
        .flat_map(move |entry| {
            use chumsky::Parser;

            let family_entry_path = entry.path();
            let family_entry_file_name = family_entry_path.file_name().unwrap().to_string_lossy();
            let family = FontFamily::parser(span)
                .parse(&family_entry_file_name)
                .into_result()
                .unwrap_or_else(|err| {
                    panic!("Parse family name at {family_entry_path:?}: {err:?}")
                });

            get_dir_entries(family_entry_path)
                .unwrap()
                .map(move |entry| {
                    let font_entry_path = entry.path();
                    let font_entry_file_name =
                        font_entry_path.file_name().unwrap().to_string_lossy();
                    let name = Ident::new(&font_entry_file_name, span);

                    let types = get_ttf_entries(entry.path())
                        .unwrap()
                        .map(|ttf_entry| {
                            let ttf_entry_path = ttf_entry.path();
                            let ttf_entry_file_name =
                                ttf_entry_path.file_name().unwrap().to_string_lossy();
                            let type_str = ttf_entry_file_name
                                .replace(&format!("{font_entry_file_name}-"), "")
                                .replace(".ttf", "");

                            FontType::parser(
                                span,
                                LitStr::new(
                                    &format!("../{}", ttf_entry.path().to_string_lossy()),
                                    span,
                                ),
                            )
                            .parse(&type_str)
                            .into_result()
                            .unwrap_or_else(|err| {
                                panic!("Parse font type at {ttf_entry_path:?}: {err:?}")
                            })
                        })
                        .collect();

                    Font {
                        family: family.clone(),
                        name,
                        types,
                    }
                })
        })
        .collect::<Vec<_>>();

    fn empty_if_normal(str: &str) -> &str {
        match str {
            "Normal" => "",
            s => s,
        }
    }

    fn constant_empty_if_normal(str: &str) -> String {
        match ccase!(constant, str).as_str() {
            "NORMAL" => "".into(),
            s => format!("_{s}"),
        }
    }

    let embedded_font_enum = {
        let names = fonts.iter().flat_map(|f| {
            f.types.iter().map(
                |FontType {
                     weight,
                     style,
                     stretch,
                     ..
                 }| {
                    format_ident!(
                        "{}{}{}{}",
                        f.name,
                        empty_if_normal(&weight.to_string()),
                        empty_if_normal(&style.to_string()),
                        empty_if_normal(&stretch.to_string()),
                    )
                },
            )
        });
        let match_arms = fonts.iter().flat_map(
            |Font {
                 name,
                 family,
                 types,
             }| {
                let const_name_base = format!(
                    "{}_{}",
                    family.const_str(),
                    ccase!(constant, name.to_string())
                );

                types.iter().map(
                    move |FontType {
                              weight,
                              style,
                              stretch,
                              ..
                          }| {
                        let weight_str = constant_empty_if_normal(&weight.to_string());
                        let style_str = constant_empty_if_normal(&style.to_string());
                        let stretch_str = constant_empty_if_normal(&stretch.to_string());

                        let const_name_font_handle = Ident::new(
                            &format!("{const_name_base}{weight_str}{style_str}{stretch_str}_FONT"),
                            weight.span(),
                        );

                        let enum_name = format_ident!(
                            "{}{}{}{}",
                            name,
                            empty_if_normal(&weight.to_string()),
                            empty_if_normal(&style.to_string()),
                            empty_if_normal(&stretch.to_string()),
                        );

                        quote! {
                            Self::#enum_name => #const_name_font_handle
                        }
                    },
                )
            },
        );

        quote! {
            #[derive(
                Debug,
                Clone,
                Copy,
                serde::Serialize,
                serde::Deserialize
            )]
            pub enum EmbeddedFont {
                #(#names),*
            }

            impl EmbeddedFont {
                pub fn to_font(self) -> iced::Font {
                    match self {
                        #(#match_arms),*
                    }
                }
            }
        }
    };

    let consts = fonts.iter().flat_map(
        |Font {
             name,
             family,
             types,
         }| {
            let const_name_base = format!(
                "{}_{}",
                family.const_str(),
                ccase!(constant, name.to_string())
            );

            types
                .iter()
                .map(
                    move |FontType {
                              weight,
                              style,
                              stretch,
                              ttf_file,
                          }| {
                        let weight_str = constant_empty_if_normal(&weight.to_string());
                        let style_str = constant_empty_if_normal(&style.to_string());
                            let stretch_str = constant_empty_if_normal(&stretch.to_string());

                        let const_name_bytes = Ident::new(
                            &format!(
                                "{const_name_base}{weight_str}{style_str}{stretch_str}_BYTES",
                            ),
                            weight.span(),
                        );
                        let const_name_font_handle = Ident::new(
                            &format!(
                                "{const_name_base}{weight_str}{style_str}{stretch_str}_FONT"
                            ),
                            weight.span(),
                        );

                        quote! {
                            pub const #const_name_bytes: &[u8] = include_bytes!(#ttf_file);
                            pub const #const_name_font_handle: iced::Font = iced::Font {
                                family: #family,
                                weight: iced::font::Weight::#weight,
                                stretch: iced::font::Stretch::#stretch,
                                style: iced::font::Style::#style
                            };
                        }
                    },
                )
                .collect::<Vec<_>>()
        },
    );

    let load_fn = {
        let const_bytes = fonts.iter().flat_map(
            |Font {
                 name,
                 family,
                 types,
             }| {
                let const_name_base = format!(
                    "{}_{}",
                    family.const_str(),
                    ccase!(constant, name.to_string())
                );

                types
                    .iter()
                    .map(
                        move |FontType {
                                  weight,
                                  style,
                                  stretch,
                                  ..
                              }| {
                            let weight_str = constant_empty_if_normal(&weight.to_string());
                            let style_str = constant_empty_if_normal(&style.to_string());
                            let stretch_str = constant_empty_if_normal(&stretch.to_string());

                            Ident::new(
                                &format!(
                                    "{const_name_base}{weight_str}{style_str}{stretch_str}_BYTES",
                                ),
                                weight.span(),
                            )
                        },
                    )
                    .collect::<Vec<_>>()
            },
        );

        quote! {
            pub fn load<P: iced::Program>(app: iced::application::Application<P>) -> iced::application::Application<P> {
                app
                    #(.font(#const_bytes))*
            }
        }
    };

    quote! {
        pub mod fonts {
            #embedded_font_enum

            #(#consts)*

            #load_fn
        }
    }
    .into()
}

struct Font {
    name: Ident,
    family: FontFamily,
    types: Vec<FontType>,
}

#[derive(Clone)]
enum FontFamily {
    Name(LitStr),
    Serif,
    SansSerif,
    Cursive,
    Fantasy,
    Monospace,
}

impl FontFamily {
    fn parser<'a>(
        span: Span,
    ) -> impl chumsky::Parser<'a, &'a str, Self, chumsky::extra::Err<Rich<'a, char>>> {
        use chumsky::prelude::*;

        choice((
            just("Serif").to(Self::Serif),
            just("SansSerif").to(Self::SansSerif),
            just("Cursive").to(Self::Cursive),
            just("Fantasy").to(Self::Fantasy),
            just("Monospace").to(Self::Monospace),
        ))
        .or(any()
            .repeated()
            .at_least(1)
            .to_slice()
            .map(move |str| Self::Name(LitStr::new(str, span))))
    }

    fn const_str(&self) -> String {
        ccase!(
            constant,
            match self {
                Self::Name(lit_str) => lit_str.value(),
                Self::Serif => "Serif".into(),
                Self::SansSerif => "SansSerif".into(),
                Self::Cursive => "Cursive".into(),
                Self::Fantasy => "Fantasy".into(),
                Self::Monospace => "Monospace".into(),
            }
        )
    }
}

impl ToTokens for FontFamily {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let variant = match self {
            Self::Name(lit_str) => quote!(Name(#lit_str)),
            Self::Serif => quote!(Serif),
            Self::SansSerif => quote!(SansSerif),
            Self::Cursive => quote!(Cursive),
            Self::Fantasy => quote!(Fantasy),
            Self::Monospace => quote!(Monospace),
        };
        tokens.extend(quote!(iced::font::Family::#variant));
    }
}

struct FontType {
    weight: Ident,
    style: Ident,
    stretch: Ident,
    ttf_file: LitStr,
}

impl FontType {
    fn parser<'a>(
        span: Span,
        ttf_file: LitStr,
    ) -> impl chumsky::Parser<'a, &'a str, Self, chumsky::extra::Err<Rich<'a, char>>> {
        use chumsky::prelude::*;

        let weight = choice((
            just("Thin"),
            just("ExtraLight"),
            just("Light"),
            just("Normal").or(just("Regular")).to("Normal"),
            just("Medium"),
            just("Semibold").or(just("SemiBold")).to("Semibold"),
            just("Bold"),
            just("ExtraBold"),
            just("Black"),
        ))
        .or_not()
        .map(|s| s.unwrap_or("Normal"))
        .map(move |w| Ident::new(w, span));
        let style = choice((just("Normal"), just("Italic"), just("Oblique")))
            .or_not()
            .map(|s| s.unwrap_or("Normal"))
            .map(move |s| Ident::new(s, span));
        let stretch = choice((
            just("UltraCondensed"),
            just("ExtraCondensed"),
            just("Condensed"),
            just("SemiCondensed"),
            just("Normal"),
            just("SemiExpanded"),
            just("Expanded"),
            just("ExtraExpanded"),
            just("UltraExpanded"),
        ))
        .or_not()
        .map(|s| s.unwrap_or("Normal"))
        .map(move |s| Ident::new(s, span));

        weight
            .then(style)
            .then(stretch)
            .map(move |((weight, style), stretch)| Self {
                weight,
                style,
                stretch,
                ttf_file: ttf_file.clone(),
            })
    }
}

fn get_dir_entries(path: impl AsRef<Path>) -> std::io::Result<impl Iterator<Item = DirEntry>> {
    Ok(std::fs::read_dir(path)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| e.path().is_dir()))
}

fn get_ttf_entries(path: impl AsRef<Path>) -> std::io::Result<impl Iterator<Item = DirEntry>> {
    Ok(std::fs::read_dir(path)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| e.path().is_file() && e.path().extension() == Some(OsStr::new("ttf"))))
}
