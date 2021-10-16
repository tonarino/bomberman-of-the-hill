use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemTrait, Pat, ReturnType, TraitItem};

pub fn implementation(input: TokenStream) -> TokenStream {
    let trait_block = parse_macro_input!(input as ItemTrait);

    let methods: Vec<_> = trait_block
        .items
        .iter()
        .filter_map(|i| if let TraitItem::Method(m) = i { Some(m) } else { None })
        .collect();

    let wrappers = methods.iter().map(|m| build_wasm_wrapper(m));

    let expanded = quote! {
        #trait_block
        #(#wrappers)*
    };

    TokenStream::from(expanded)
}

fn build_wasm_wrapper(method: &syn::TraitItemMethod) -> quote::__private::TokenStream {
    let wrapper_identifier = format_ident!("wasm_{}", method.sig.ident.clone());
    let shim_identifier = format!("__wasm_shim_{}", method.sig.ident.clone());

    let (valid_inputs, input_patterns): (Vec<_>, Vec<_>) = method
        .sig
        .inputs
        .iter()
        .filter_map(|i| {
            if let syn::FnArg::Typed(t) = i {
                if let Pat::Ident(id) = &*t.pat {
                    Some((i.clone(), id))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unzip();

    let shim_input_addresses: Vec<_> =
        input_patterns.iter().map(|p| format_ident!("{}_address", p.ident)).collect();
    let shim_input_lengths: Vec<_> =
        input_patterns.iter().map(|p| format_ident!("{}_length", p.ident)).collect();
    let shim_input_types = (0..valid_inputs.len() * 2).map(|_| format_ident!("i32"));

    let shim_output_type = if matches!(method.sig.output, ReturnType::Type(..)) {
        format_ident!("i32")
    } else {
        format_ident!("()")
    };

    let input_processing = quote! {

        let memory = instance.get_memory(store.as_context_mut(), "memory")
            .ok_or(anyhow::anyhow!("Wasm memory block not found"))?;
        let get_wasm_buffer_address = instance.get_typed_func::<(), i32, _>(
            store.as_context_mut(), "__wasm_get_buffer_address"
        )?;
        let get_wasm_buffer_size = instance.get_typed_func::<(), i32, _>(
            store.as_context_mut(), "__wasm_get_buffer_size"
        )?;
        let wasm_buffer_base_address = get_wasm_buffer_address.call(store.as_context_mut(), ())?;
        let wasm_buffer_size = get_wasm_buffer_size.call(store.as_context_mut(), ())? as usize;
        let mut wasm_buffer_address = wasm_buffer_base_address;

        #(
            let #input_patterns = bincode::serialize(&#input_patterns)?;
            let #shim_input_addresses = wasm_buffer_address as usize;
            let #shim_input_lengths = #input_patterns.as_slice().len();
            let buffer_space_required = #shim_input_addresses.saturating_sub(wasm_buffer_base_address as usize) + #shim_input_lengths;
            if buffer_space_required > wasm_buffer_size {
                return Err(anyhow::anyhow!("Wasm method inputs too big for the `wasm` buffer"));
            }
            memory.write(store.as_context_mut(), #shim_input_addresses, #input_patterns.as_slice())?;
            wasm_buffer_address += #shim_input_lengths as i32;
        )*

        let method = instance.get_typed_func::<(#(#shim_input_types),*), #shim_output_type, _>(store.as_context_mut(), #shim_identifier)?;
    };

    let expanded = if let ReturnType::Type(_, ref output) = method.sig.output {
        quote! {
            #[cfg(not(target_family = "wasm"))]
            pub fn #wrapper_identifier(
                store: &mut ::wasmtime::Store<()>,
                instance: & ::wasmtime::Instance,
                #(#valid_inputs),*
            ) -> ::anyhow::Result<#output> {

                #input_processing
                let return_length = method.call(store.as_context_mut(),(#(#shim_input_addresses as _,)* #(#shim_input_lengths as _),*))?;

                let mut dynamic_buffer = vec![0u8; return_length as usize];
                memory.read(store.as_context_mut(), wasm_buffer_base_address as usize, dynamic_buffer.as_mut_slice())?;
                let result = bincode::deserialize(dynamic_buffer.as_slice())?;
                Ok(result)
            }
        }
    } else {
        quote! {
            #[cfg(not(target_family = "wasm"))]
            pub fn #wrapper_identifier(
                store: &mut ::wasmtime::Store<()>,
                instance: & ::wasmtime::Instance,
                #(#valid_inputs),*
            ) {
                #input_processing
                method.call(store.as_context_mut(),(#(#shim_input_addresses as _, #shim_input_lengths as _),*))?;
                Ok(())
            }
        }
    };

    expanded
}
