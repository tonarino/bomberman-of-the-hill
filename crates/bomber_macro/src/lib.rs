use proc_macro::TokenStream;

mod wasm_export;
mod wasm_wrap;

#[proc_macro_attribute]
pub fn wasm_export(_: TokenStream, input: TokenStream) -> TokenStream {
    wasm_export::implementation(input)
}

#[proc_macro_attribute]
pub fn wasm_wrap(_: TokenStream, input: TokenStream) -> TokenStream {
    wasm_wrap::implementation(input)
}
