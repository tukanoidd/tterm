use std::{ffi::OsStr, fs::DirEntry, path::Path};

use chumsky::prelude::*;
use convert_case::ccase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, format_ident, quote};
use syn::{Ident, LitStr, parse_macro_input};

pub fn generate(input: TokenStream) -> TokenStream {
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
