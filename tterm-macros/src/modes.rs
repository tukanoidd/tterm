use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Field, FieldsNamed, Ident, Token, Variant, braced, parse::Parse};

use crate::modes::actions::Actions;

mod actions;

pub struct Modes {
    types: Vec<Ident>,
}

impl Modes {
    pub fn generate(self) -> TokenStream {
        let Self { types } = self;

        quote! {
            #[derive(Debug, derive_more::Display, Clone, Copy, PartialEq)]
            pub enum TTermModeVariant {
                #(#types),*
            }
        }
    }
}

impl Parse for Modes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let types = input
            .parse_terminated(Ident::parse, Token![,])?
            .into_iter()
            .collect();

        Ok(Self { types })
    }
}

pub struct Mode {
    name: Ident,

    message: ModeMessage,
    actions: Actions,

    config: ModeConfig,
    state: ModeState,

    impl_fns: Vec<syn::ItemFn>,
}

impl Mode {
    pub fn generate(self) -> TokenStream {
        let Self {
            name,
            message,
            actions,
            config,
            state,
            impl_fns,
        } = self;

        let struct_name = format_ident!("{name}Mode");
        let message_name = ModeMessage::enum_name(&name);
        let config_name = ModeConfig::struct_name(&name);
        let keybind_panel_type_enum_name = format_ident!("{name}ModeKeyBindPanelType");
        let action_name = Actions::enum_name(&name);
        let state_name = ModeState::struct_name(&name);

        let impl_fns = impl_fns.into_iter().map(|f| f.to_token_stream());

        let message_enum = message.generate_enum(&name);
        let action_tokens = actions.generate_enum(&name);

        let state_struct = state.generate_struct(&name);

        let config_struct = config.generate_struct(&name);

        quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            pub struct #struct_name;

            impl crate::app::mode::TTermMode for #struct_name {
                type Message = #message_name;
                type KeyBindPanelType = #keybind_panel_type_enum_name;
                type Action = #action_name;
                type Config = #config_name;

                type State = #state_name;

                #(#impl_fns)*
            }

            #action_tokens
            #message_enum

            #state_struct

            #config_struct
        }
    }
}

impl Parse for Mode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;

        let data_input;
        let _br = braced!(data_input in input);

        // message
        let message_ident: Ident = data_input.parse()?;

        if message_ident != "message" {
            return Err(syn::Error::new(message_ident.span(), "Not 'message'"));
        }

        let _col: Token![:] = data_input.parse()?;
        let message: ModeMessage = data_input.parse()?;
        let _semi: Token![;] = data_input.parse()?;

        // actions
        let actions_ident: Ident = data_input.parse()?;

        if actions_ident != "actions" {
            return Err(syn::Error::new(actions_ident.span(), "Not 'actions'"));
        }

        let _col: Token![:] = data_input.parse()?;
        let actions: Actions = data_input.parse()?;
        let _semi: Token![;] = data_input.parse()?;

        // config
        let config_ident: Ident = data_input.parse()?;

        if config_ident != "config" {
            return Err(syn::Error::new(config_ident.span(), "Not 'config'"));
        }

        let _col: Token![:] = data_input.parse()?;
        let config: ModeConfig = data_input.parse()?;
        let _semi: Token![;] = data_input.parse()?;

        // states
        let state_ident: Ident = data_input.parse()?;

        if state_ident != "state" {
            return Err(syn::Error::new(config_ident.span(), "Not 'state'"));
        }

        let _col: Token![:] = data_input.parse()?;

        let state: ModeState = data_input.parse()?;
        let _semi: Token![;] = data_input.parse()?;

        let impl_fns = vec![
            data_input.parse()?,
            data_input.parse()?,
            data_input.parse()?,
        ];

        Ok(Self {
            name,

            message,
            actions,

            config,
            state,

            impl_fns,
        })
    }
}

struct ModeMessage {
    variants: Vec<Variant>,
}

impl ModeMessage {
    fn enum_name(mode: &Ident) -> Ident {
        format_ident!("{mode}ModeMessage")
    }

    fn generate_enum(self, mode: &Ident) -> TokenStream {
        let Self { variants } = self;

        let enum_name = Self::enum_name(mode);
        let action_enum_name = Actions::enum_name(mode);

        let mode_struct_name = format_ident!("{mode}Mode");
        let keybind_panel_type_enum_name = format_ident!("{mode}ModeKeyBindPanelType");
        let panel_toggle_var = quote!(PanelToggle {
            ty: #keybind_panel_type_enum_name,
            force: Option<bool>
        });
        let action_var = quote!(Action(#action_enum_name));

        quote! {
            #[derive(Debug, Clone, derive_more::From)]
            pub enum #enum_name {
                #action_var,
                #panel_toggle_var,
                #(#variants),*
            }

            impl crate::app::mode::TTermModeMessage<#mode_struct_name> for #enum_name {
                fn panel_toggle(
                    ty: <#mode_struct_name as crate::app::mode::TTermMode>::KeyBindPanelType,
                    force: Option<bool>
                ) -> Self {
                    Self::PanelToggle { ty, force }
                }
            }
        }
    }
}

impl Parse for ModeMessage {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let list_input;
        let _br = braced!(list_input in input);

        let variants = list_input
            .parse_terminated(Variant::parse, Token![,])?
            .into_iter()
            .collect::<Vec<_>>();

        Ok(Self { variants })
    }
}

struct ModeConfig {
    fields: FieldsNamed,
}

impl ModeConfig {
    fn struct_name(mode: &Ident) -> Ident {
        format_ident!("{mode}ModeConfig")
    }

    fn generate_struct(self, mode: &Ident) -> TokenStream {
        let Self { fields } = self;
        let struct_name = Self::struct_name(mode);

        let mode_struct_name = format_ident!("{mode}Mode");

        let fields = fields
            .named
            .into_iter()
            .map(|Field { ident, ty, .. }| quote!(pub #ident: #ty))
            .chain([
                quote!(pub keybinds: crate::config::keybinds::KeyBindsConfig<#mode_struct_name>),
            ]);

        quote! {
            #[derive(
                smart_default::SmartDefault,
                Debug,
                Clone,
                serde::Serialize, serde::Deserialize
            )]
            #[serde(default)]
            pub struct #struct_name {
                #(#fields),*
            }

            impl crate::app::mode::TTermModeConfig<#mode_struct_name> for #struct_name {
                fn keybinds(&self) -> &crate::config::keybinds::KeyBindsConfig<#mode_struct_name> {
                    &self.keybinds
                }
            }
        }
    }
}

impl Parse for ModeConfig {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let fields: FieldsNamed = input.parse()?;

        Ok(Self { fields })
    }
}

struct ModeState {
    fields: FieldsNamed,
}

impl ModeState {
    fn struct_name(mode: &Ident) -> Ident {
        format_ident!("{mode}ModeState")
    }

    fn generate_struct(self, mode: &Ident) -> TokenStream {
        let Self { fields, .. } = self;

        let mode_struct_name = format_ident!("{mode}Mode");
        let struct_name = Self::struct_name(mode);
        let fields = fields.named.into_iter().map(|Field { ident, ty, .. }| {
            quote! {
                pub #ident: #ty
            }
        });

        quote! {
            pub struct #struct_name {
                pub keybind_panel_expanded: std::collections::HashMap<
                    <#mode_struct_name as crate::app::TTermMode>::KeyBindPanelType,
                    bool
                >,
                #(#fields),*
            }

            impl #struct_name {
                pub fn panel_toggle(
                    &mut self,
                    ty: <#mode_struct_name as crate::app::TTermMode>::KeyBindPanelType,
                    force: Option<bool>
                ) {
                    let entry = self.keybind_panel_expanded.entry(ty).or_default();
                    *entry = force.unwrap_or(!*entry);
                }
            }
        }
    }
}

impl Parse for ModeState {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let fields: FieldsNamed = input.parse()?;

        Ok(Self { fields })
    }
}
