mod fonts;
mod modes;

use std::io::prelude::Write;

use proc_macro::TokenStream;

#[proc_macro]
pub fn modes(input: TokenStream) -> TokenStream {
    let output = modes::generate(input);

    // let macro_test = std::path::Path::new("macro-test");

    // if !macro_test.exists() {
    //     std::fs::create_dir_all(macro_test).unwrap();
    // }

    // let mut dump_file = std::fs::File::create(macro_test.join("modes.rs")).unwrap();
    // dump_file
    //     .write_all(&output.to_string().into_bytes())
    //     .unwrap();

    output
}

#[proc_macro]
pub fn fonts(input: TokenStream) -> TokenStream {
    fonts::generate(input)
}
