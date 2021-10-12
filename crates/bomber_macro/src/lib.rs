use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_attribute]
pub fn wasm_hero(_: TokenStream, input: TokenStream) -> TokenStream {
    let hero_struct = parse_macro_input!(input as ItemStruct);
    let hero_identifier = hero_struct.ident.clone();

    let expanded = quote! {
        #hero_struct

        lazy_static::lazy_static! {
            static ref __HERO: std::sync::Mutex<#hero_identifier> = std::sync::Mutex::new(#hero_identifier::spawn());
        }

        struct __WorldShim;

        impl World for __WorldShim {
            fn inspect(&self, direction: Direction) -> Tile {
                unsafe { __inspect(direction as u32).into() }
            }
        }

        #[no_mangle]
        pub fn __act() -> u32 {
            __HERO.lock().unwrap().act(&__WorldShim).into()
        }

        extern { fn __inspect(direction_raw: u32) -> u32; }
    };

    proc_macro::TokenStream::from(expanded)
}
