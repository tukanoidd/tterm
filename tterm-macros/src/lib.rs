use convert_case::ccase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, LitStr, Token, braced, parenthesized, parse::Parse, parse_macro_input};

#[proc_macro]
pub fn fonts(input: TokenStream) -> TokenStream {
    let FontList { list } = parse_macro_input!(input);

    let embedded_font_enum = {
        let names = list.iter().map(|f| &f.name);

        quote! {
            #[derive(
                Debug,
                Clone,
                serde::Serialize,
                serde::Deserialize
            )]
            pub enum EmbeddedFont {
                #(#names),*
            }
        }
    };

    let consts = list.into_iter().flat_map(
        |Font {
             name,
             path,
             prefix,
             family,
             weights: types,
         }| {
            let const_name_base = ccase!(constant, name.to_string());
            let path_base = format!("../assets/fonts/{}/{}-", path.value(), prefix.value());

            types
                .iter()
                .map(move |weight| {
                    let weight_str = ccase!(constant, weight.to_string());
                    let const_name_bytes = Ident::new(
                        &format!("{const_name_base}_{weight_str}_BYTES",),
                        weight.span(),
                    );
                    let path = LitStr::new(&format!("{path_base}{weight}.ttf"), weight.span());

                    let const_name_font_handle = Ident::new(
                        &format!("{const_name_base}_{weight_str}_FONT"),
                        weight.span(),
                    );

                    quote! {
                        const #const_name_bytes: &[u8] = include_bytes!(#path);
                        const #const_name_font_handle: iced::Font = iced::Font {
                            family: iced::font::Family::#family,
                            weight: iced::font::Weight::#weight,
                            stretch: iced::font::Stretch::Normal,
                            style: iced::font::Style::Normal
                        };
                    }
                })
                .collect::<Vec<_>>()
        },
    );

    quote! {
        #embedded_font_enum

        #(#consts)*
    }
    .into()
}

struct FontList {
    list: Vec<Font>,
}

impl Parse for FontList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let list = input
            .parse_terminated(Font::parse, Token![,])?
            .into_iter()
            .collect();

        Ok(Self { list })
    }
}

struct Font {
    name: Ident,
    path: LitStr,
    prefix: LitStr,
    family: Ident,
    weights: Vec<Ident>,
}

impl Parse for Font {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let attr_input;
        let _paren = parenthesized!(attr_input in input);
        let path: LitStr = attr_input.parse()?;
        let _eq: Token![=>] = attr_input.parse()?;
        let prefix: LitStr = attr_input.parse()?;

        let _at: Token![@] = attr_input.parse()?;
        let font_info_input;
        let _brace = braced!(font_info_input in attr_input);
        let family: Ident = font_info_input.parse()?;

        let _colon: Token![:] = input.parse()?;

        let types_input;
        let _brace = braced!(types_input in input);
        let weights = types_input
            .parse_terminated(Ident::parse, Token![,])?
            .into_iter()
            .collect();

        Ok(Self {
            name,
            path,
            prefix,
            family,
            weights,
        })
    }
}
