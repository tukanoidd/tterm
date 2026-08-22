mod fonts;
mod modes;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn modes(input: TokenStream) -> TokenStream {
    let modes: modes::Modes = parse_macro_input!(input);
    modes.generate().into()
}

#[proc_macro]
pub fn mode(input: TokenStream) -> TokenStream {
    let mode: modes::Mode = parse_macro_input!(input);
    mode.generate().into()
}

#[proc_macro]
pub fn fonts(input: TokenStream) -> TokenStream {
    fonts::generate(input)
}
