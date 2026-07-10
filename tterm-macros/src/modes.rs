use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Field, FieldsNamed, Ident, Token, Variant, braced, bracketed, parse::Parse, parse_macro_input,
};

use crate::modes::actions::Actions;

mod actions;

pub fn generate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let modes: Modes = parse_macro_input!(input);
    modes.generate().into()
}

pub struct Modes {
    types: Vec<Mode>,
}

impl Modes {
    pub fn generate(self) -> TokenStream {
        let Self { types } = self;
        let variant_enum = Self::mode_variant_enum(&types);

        let impls = types.into_iter().map(Mode::generate_struct_impls);

        quote! {
            #variant_enum

            #(#impls)*
        }
    }

    fn mode_variant_enum(types: &[Mode]) -> TokenStream {
        let vars = types.iter().map(|mode| &mode.name);

        quote! {
            #[derive(Debug, derive_more::Display, Clone, Copy, PartialEq)]
            pub enum TTermModeVariant {
                #(#vars),*
            }
        }
    }
}

impl Parse for Modes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let types = input
            .parse_terminated(Mode::parse, Token![,])?
            .into_iter()
            .collect();

        Ok(Self { types })
    }
}

struct Mode {
    name: Ident,

    message: ModeMessage,
    actions: Actions,

    config: ModeConfig,
    states: [ModeState; 3],

    impl_fns: Vec<syn::ItemFn>,
}

impl Mode {
    fn generate_struct_impls(self) -> TokenStream {
        let Self {
            name,
            message,
            actions,
            config,
            states,
            impl_fns,
        } = self;

        let struct_name = format_ident!("{name}Mode");
        let message_name = ModeMessage::enum_name(&name);
        let config_name = ModeConfig::struct_name(&name);
        let view_state_name = format_ident!("{name}ModeViewState");
        let update_state_name = format_ident!("{name}ModeUpdateState");
        let subscription_state_name = format_ident!("{name}ModeSubscriptionState");

        let impl_fns = impl_fns.into_iter().map(|f| f.to_token_stream());

        let state_structs = states.into_iter().map(|s| s.generate_struct(&name));

        let message_enum = message.generate_enum(&name);
        let action_tokens = actions.generate_enum(&name);

        let config_struct = config.generate_struct(&name);

        quote! {
            pub struct #struct_name;

            impl<'a> TTermMode<'a> for #struct_name {
                type Message = #message_name;

                type Config = #config_name;

                type ViewState = #view_state_name<'a>;
                type UpdateState = #update_state_name<'a>;
                type SubscriptionState = #subscription_state_name<'a>;

                #(#impl_fns)*
            }

            #(#state_structs)*

            #action_tokens
            #message_enum

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
        let states_ident: Ident = data_input.parse()?;

        if states_ident != "states" {
            return Err(syn::Error::new(config_ident.span(), "Not 'states'"));
        }

        let _col: Token![:] = data_input.parse()?;

        let states_input;
        let _br = bracketed!(states_input in data_input);
        let mut states_list = states_input
            .parse_terminated(ModeState::parse, Token![,])?
            .into_iter()
            .collect::<Vec<_>>();

        let states = match states_list.len() {
            3 => [
                states_list.remove(0),
                states_list.remove(0),
                states_list.remove(0),
            ],
            l if l < 3 => {
                let last = states_list.last();

                return Err(syn::Error::new(
                    last.as_ref()
                        .map(|m| m.span.clone())
                        .unwrap_or_else(|| data_input.span()),
                    match last.as_ref() {
                        Some(_) => format!(
                            "Only {l} states out of [view, update, subscription] were provided"
                        ),
                        None => {
                            "None of the states out of [view, update, subscription] were provided"
                                .into()
                        }
                    },
                ));
            }
            l => {
                let last = states_list.last().unwrap();
                return Err(syn::Error::new(
                    last.span,
                    format!(
                        "Too many ({l}) states were provided out of [view, update, subscription]"
                    ),
                ));
            }
        };
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
            states,

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
        let action_var = quote!(Action(#action_enum_name));

        quote! {
            #[derive(Debug, Clone, derive_more::From)]
            pub enum #enum_name {
                #[from(skip)]
                #action_var,
                #(#variants),*
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

        let keybinds_config_struct_name = format_ident!("{mode}ModeKeyBindsConfig");
        let action_enum_name = Actions::enum_name(mode);
        let keybinds_config_struct = {
            quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                pub struct #keybinds_config_struct_name {
                    pub actions: Vec<(crate::config::keybinds::KeyBind, #action_enum_name)>
                }

                impl Default for #keybinds_config_struct_name {
                    fn default() -> Self {
                        Self {
                            actions: #action_enum_name::default_keybinds()
                                .into_iter()
                                .flat_map(|(_, map)| map.into_iter())
                                .collect(),
                        }
                    }
                }
            }
        };
        let fields = fields
            .named
            .into_iter()
            .map(|Field { ident, ty, .. }| quote!(#ident: #ty))
            .chain([quote!(keybinds: #keybinds_config_struct_name)]);

        quote! {
            #[derive(
                smart_default::SmartDefault,
                Debug,
                Clone,
                serde::Serialize, serde::Deserialize
            )]
            pub struct #struct_name {
                #(#fields),*
            }

            #keybinds_config_struct
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
    span: Span,
    ty: ModeStateType,
    fields: FieldsNamed,
}

impl ModeState {
    fn generate_struct(self, mode: &Ident) -> TokenStream {
        let Self { ty, fields, .. } = self;

        let ref_ = match &ty {
            ModeStateType::View | ModeStateType::Subscription => quote!(&'a),
            ModeStateType::Update => quote!(&'a mut),
        };
        let struct_name = ty.struct_name(mode);
        let field_names = fields
            .named
            .iter()
            .map(|f| f.ident.clone())
            .collect::<Vec<_>>();
        let fields = fields.named.into_iter().map(|Field { ident, ty, .. }| {
            quote! {
                #ident: #ref_ #ty
            }
        });

        quote! {
            pub struct #struct_name<'a> {
                #(#fields),*
            }

            impl<'a> From<#ref_ MainState> for #struct_name<'a> {
                fn from(MainState {
                    #(#field_names,)*
                    ..
                }: #ref_ MainState) -> Self {
                    Self {
                        #(#field_names),*
                    }
                }
            }
        }
    }
}

impl Parse for ModeState {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let ty: ModeStateType = input.parse()?;
        let fields: FieldsNamed = input.parse()?;

        Ok(Self { span, ty, fields })
    }
}

enum ModeStateType {
    View,
    Update,
    Subscription,
}

impl ModeStateType {
    fn struct_name(self, mode: &Ident) -> Ident {
        format_ident!(
            "{mode}Mode{}State",
            match self {
                Self::View => "View",
                Self::Update => "Update",
                Self::Subscription => "Subscription",
            },
        )
    }
}

impl Parse for ModeStateType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _at: Token![@] = input.parse()?;
        let name: Ident = input.parse()?;

        let var = [
            ("view", ModeStateType::View),
            ("update", ModeStateType::Update),
            ("subscription", ModeStateType::Subscription),
        ]
        .into_iter()
        .find_map(|(n, v)| (name == n).then_some(v))
        .ok_or_else(|| syn::Error::new(name.span(), "Neither view, update or subscription"))?;

        Ok(var)
    }
}
