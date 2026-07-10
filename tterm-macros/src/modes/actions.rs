use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident, LitStr, Token, Variant, braced, bracketed, parse::Parse};

use crate::modes::ModeMessage;

pub struct Actions {
    types: Vec<ActionType>,
}

impl Actions {
    pub fn enum_name(mode: &Ident) -> Ident {
        format_ident!("{mode}ModeAction")
    }

    pub fn generate_enum(self, mode: &Ident) -> TokenStream {
        let Self { types } = self;

        let tterm_action_enum = {
            let variants = types
                .iter()
                .map(|ActionType { name, .. }| {
                    let type_name = format_ident!("{mode}Mode{name}Action");
                    quote!(#name(#type_name))
                })
                .collect::<Vec<_>>();

            let enum_name = Self::enum_name(mode);

            let keybind_panel_type_enum_name = format_ident!("{mode}ModeKeyBindPanelType");
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
                    pub enum #keybind_panel_type_enum_name {
                        #(#variants),*
                    }

                    impl #keybind_panel_type_enum_name {
                        pub fn title(&self) -> String {
                            match self {
                                #(#title_match_arms),*
                            }
                        }
                    }

                    impl<'a> From<&'a #enum_name> for #keybind_panel_type_enum_name {
                        fn from(value: &'a #enum_name) -> Self {
                            match value {
                                #(#enum_name::#variants(_) => Self::#variants),*
                            }
                        }
                    }
                }
            };

            let impls = {
                let default_keybinds = {
                    let binds = types.iter().map(|ActionType { name, .. }| {
                        let type_name = format_ident!("{mode}Mode{name}Action");

                        quote!((
                            #keybind_panel_type_enum_name::#name,
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
                        fn default_keybinds() -> Vec<(
                            #keybind_panel_type_enum_name,
                            Vec<(crate::config::keybinds::KeyBind, #enum_name)>
                        )> {
                            Vec::from_iter([#(#binds),*])
                        }
                    }
                };

                quote! {
                    impl #enum_name {
                        #default_keybinds
                    }
                }
            };

            quote! {
                #[derive(
                    Debug, derive_more::Display,
                    Clone, Hash,
                    derive_more::From,
                    serde::Serialize, serde::Deserialize
                )]
                pub enum #enum_name {
                    #(#variants),*
                }

                #impls

                #keybind_panel_type_enum
            }
        };

        let types = types.iter().map(|ActionType { name, actions }| {
            let type_name = format_ident!("{mode}Mode{name}Action");
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
                                    Key::Named(ident) => quote!(
                                        crate::config::keybinds::Key::Named(crate::config::keybinds::NamedKey::#ident)
                                    ),
                                    Key::Character(lit_str) => {
                                        quote!(crate::config::keybinds::Key::Character(#lit_str.into()))
                                    }
                                };
                                let modifiers = modifiers.iter().map(|m| quote!(
                                    crate::config::keybinds::Modifier::#m
                                ));

                                quote! {(
                                    crate::config::keybinds::KeyBind::new(#key, [#(#modifiers),*]),
                                    Self::#ident #as_action
                                )}
                            },
                        )
                        },
                    );

                    quote! {
                        pub fn default_keybinds() -> impl IntoIterator<Item = (
                            crate::config::keybinds::KeyBind,
                            #type_name
                        )> {
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

            let from_trait = {
                let message = ModeMessage::enum_name(mode);
                
                quote! {
                    impl<IA> From<IA> for AppMsg where IA: Into<#type_name> {
                        fn from(value: IA) -> AppMsg {
                            #message::Action(value.into()).into()
                        }
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

                #from_trait
            }
        });

        quote! {
            #tterm_action_enum

            #(#types)*
        }
    }
}

impl Parse for Actions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let list_input;
        let _br = braced!(list_input in input);

        let types = list_input
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
