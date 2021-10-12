use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_attribute]
pub fn wasm_player(_: TokenStream, input: TokenStream) -> TokenStream {
    let player_struct = parse_macro_input!(input as ItemStruct);
    let player_identifier = player_struct.ident.clone();

    let expanded = quote! {
        #player_struct

        lazy_static::lazy_static! {
            static ref __PLAYER: std::sync::Mutex<#player_identifier> = std::sync::Mutex::new(#player_identifier::spawn());
        }

        struct __WorldShim;

        impl World for __WorldShim {
            fn inspect(&self, direction: Direction) -> Tile {
                unsafe { __inspect(direction as u32).into() }
            }
        }

        #[no_mangle]
        pub fn __act() -> u32 {
            __PLAYER.lock().unwrap().act(&__WorldShim).into()
        }

        extern { fn __inspect(direction_raw: u32) -> u32; }
    };

    proc_macro::TokenStream::from(expanded)
}
