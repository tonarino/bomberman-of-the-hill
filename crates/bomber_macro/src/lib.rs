use proc_macro::TokenStream;

mod wasm_export;
mod wasm_wrap;

/// `wasm_export` must decorate the `impl` block for the trait
/// used as a `wasm` interface, at which point it will generate shims
/// for each of the methods, delegating to a singleton for the methods
/// with a Self receiver.
#[proc_macro_attribute]
pub fn wasm_export(_: TokenStream, input: TokenStream) -> TokenStream {
    wasm_export::implementation(input)
}

/// `wasm_wrap` must decorate the trait definition for the trait used
/// as a `wasm` interface. This will generate accesor helper functions
/// to interact with the wasm module without the need for manual type
/// conversions.
#[proc_macro_attribute]
pub fn wasm_wrap(_: TokenStream, input: TokenStream) -> TokenStream {
    wasm_wrap::implementation(input)
}
