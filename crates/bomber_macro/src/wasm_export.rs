use proc_macro::TokenStream;

use quote::{format_ident, quote};
use syn::{parse_macro_input, Ident, ImplItem, ImplItemMethod, ItemImpl, ReturnType, Type};

const BUFFER_SIZE_BYTES: usize = 10_000;

struct MethodData {
    method_identifier: Ident,
    shim_identifier: Ident,
    takes_self: bool,
    has_output: bool,
}

struct SignatureData {
    argument_identifiers: Vec<Ident>,
    pointer_identifiers: Vec<Ident>,
    length_identifiers: Vec<Ident>,
    slice_identifiers: Vec<Ident>,
}

pub fn implementation(input: TokenStream) -> TokenStream {
    let trait_impl_block = parse_macro_input!(input as ItemImpl);
    let methods: Vec<_> = trait_impl_block
        .items
        .iter()
        .filter_map(|i| if let ImplItem::Method(m) = i { Some(m) } else { None })
        .collect();
    let implementer = &trait_impl_block.self_ty;

    let mut expanded = proc_macro::TokenStream::from(quote! {
        #trait_impl_block

        /// A default lazy static instance of the trait implementer becomes
        /// the state of the `wasm` module.
        lazy_static::lazy_static! {
            static ref __WASM_SINGLETON: std::sync::Mutex<#implementer> = std::sync::Mutex::new(#implementer::default());
        }

        #[no_mangle]
        static mut __WASM_BUFFER: [u8; #BUFFER_SIZE_BYTES] = [0u8; #BUFFER_SIZE_BYTES];

        #[no_mangle]
        fn __wasm_get_buffer_address() -> i32 { unsafe { __WASM_BUFFER.as_ptr() as _ } }
        #[no_mangle]
        fn __wasm_get_buffer_size() -> i32 { #BUFFER_SIZE_BYTES as _ }
    });

    for method in methods {
        expanded.extend(build_shim(method, implementer));
    }

    expanded
}

fn build_shim(method: &ImplItemMethod, implementer: &Type) -> TokenStream {
    let MethodData { method_identifier, shim_identifier, takes_self, has_output } =
        reflect_on_method(method);
    let SignatureData {
        argument_identifiers,
        pointer_identifiers,
        length_identifiers,
        slice_identifiers,
    } = reflect_on_signature(method);

    let shim_reconstruction = quote! {
        #(
            let #slice_identifiers = unsafe { ::std::slice::from_raw_parts(#pointer_identifiers as _, #length_identifiers as _) };
            let #argument_identifiers =
                bomber_lib::bincode::deserialize(#slice_identifiers)
                .expect("Failed to deserialize argument");
         )*
    };

    let inner_invocation = inner_invocation(
        takes_self,
        has_output,
        method_identifier,
        argument_identifiers.into_iter(),
        implementer,
    );

    let expanded = if has_output {
        quote! {
            #[no_mangle]
            pub fn #shim_identifier(#(#pointer_identifiers: i32,)* #(#length_identifiers: u32),*) -> i32 {
                #shim_reconstruction
                #inner_invocation
                let serialized_output = bomber_lib::bincode::serialize(&output).expect("Failed to serialize output");
                assert!( unsafe { __WASM_BUFFER.len() >= serialized_output.len() } );
                unsafe { __WASM_BUFFER.iter_mut().zip(serialized_output.iter()).for_each(|(o, i)| *o = *i); }
                serialized_output.len() as i32
            }
        }
    } else {
        quote! {
            #[no_mangle]
            pub fn #shim_identifier(#(#pointer_identifiers: i32,)* #(#length_identifiers: u32),*) {
                #shim_reconstruction
                #inner_invocation
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}

fn reflect_on_signature(method: &ImplItemMethod) -> SignatureData {
    let inputs: Vec<_> = method.sig.inputs.iter().collect();
    let non_self_input_types = inputs.iter().filter_map(|i| match i {
        syn::FnArg::Typed(t) => Some(t),
        _ => None,
    });
    let indices = 0..non_self_input_types.count();
    let argument_identifiers: Vec<_> =
        indices.clone().map(|i| format_ident!("reconstructed_argument_{}", i)).collect();
    let pointer_identifiers: Vec<_> =
        indices.clone().map(|i| format_ident!("argument_pointer_{}", i)).collect();
    let length_identifiers: Vec<_> =
        indices.clone().map(|i| format_ident!("argument_length_{}", i)).collect();
    let slice_identifiers = indices.map(|i| format_ident!("argument_slice_{}", i)).collect();
    SignatureData {
        argument_identifiers,
        pointer_identifiers,
        length_identifiers,
        slice_identifiers,
    }
}

fn reflect_on_method(method: &ImplItemMethod) -> MethodData {
    let method_identifier = method.sig.ident.clone();
    let shim_identifier = format_ident!("__wasm_shim_{}", method.sig.ident);
    let takes_self = method.sig.inputs.iter().any(|i| matches!(i, syn::FnArg::Receiver(_)));
    let has_output = matches!(method.sig.output, ReturnType::Type(..));
    MethodData { method_identifier, shim_identifier, takes_self, has_output }
}

fn inner_invocation(
    takes_self: bool,
    has_output: bool,
    method_identifier: syn::Ident,
    argument_identifiers: impl Iterator<Item = Ident>,
    implementer: &Type,
) -> quote::__private::TokenStream {
    match (takes_self, has_output) {
        (true, false) => {
            quote! { __WASM_SINGLETON.lock().unwrap().#method_identifier(#(#argument_identifiers),*); }
        },
        (false, false) => quote! { #implementer::#method_identifier(#(#argument_identifiers),*); },
        (true, true) => quote! {
            let output = __WASM_SINGLETON.lock().unwrap().#method_identifier(#(#argument_identifiers),*);
        },
        (false, true) => quote! {
            let output = #implementer::#method_identifier(#(#argument_identifiers),*);
        },
    }
}
