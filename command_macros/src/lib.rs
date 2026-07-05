/// Attribute macro for defining commands
/// This creates a companion function that accepts a serde_json::Value Object as the parameters
/// and attempts to correctly deserialise and map them to the original function's arguments.
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{ItemFn, Path, Token, parse_macro_input};

#[proc_macro_attribute]
pub fn command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let original = input.clone(); // keep the real function exactly as written

    let vis = &input.vis;
    let sig = &input.sig;
    let fn_name = &sig.ident;
    let wrapper_name = syn::Ident::new(&format!("__cmd__{}", fn_name), fn_name.span());
    let is_async = sig.asyncness.is_some();

    fn is_str_ref(ty: &syn::Type) -> bool {
        if let syn::Type::Reference(r) = ty {
            if let syn::Type::Path(p) = &*r.elem {
                return p.path.is_ident("str");
            }
        }
        false
    }

    let mut let_stmts = Vec::new();
    let mut arg_names = Vec::new(); // for validation + as call arguments
    let mut call_args = Vec::new();

    for input in sig.inputs.iter() {
        match input {
            syn::FnArg::Typed(pat_type) => {
                let name = match &*pat_type.pat {
                    syn::Pat::Ident(i) => i.ident.clone(),
                    _ => {
                        return syn::Error::new_spanned(
                            &pat_type.pat,
                            "unsupported argument pattern",
                        )
                            .to_compile_error()
                            .into();
                    }
                };

                let ty = &pat_type.ty;
                let key = name.to_string();

                if is_str_ref(ty) {
                    let owned_ident =
                        syn::Ident::new(&format!("__owned_{}", name), name.span());
                    let_stmts.push(quote! {
                        let #owned_ident: String = serde_json::from_value(
                            obj.get(#key).unwrap().clone()
                        ).map_err(|e| e.to_string())?;
                    });
                    call_args.push(quote! { &#owned_ident });
                } else {
                    let_stmts.push(quote! {
                        let #name: #ty = serde_json::from_value(
                            obj.get(#key).unwrap().clone()
                        ).map_err(|e| e.to_string())?;
                    });
                    call_args.push(quote! { #name });
                }

                arg_names.push(name);
            }
            _ => {
                return syn::Error::new_spanned(input, "unsupported receiver type")
                    .to_compile_error()
                    .into();
            }
        }
    }

    let returns_result = match &sig.output {
        syn::ReturnType::Type(_, ty) => {
            if let syn::Type::Path(p) = &**ty {
                p.path.segments.last().unwrap().ident == "Result"
            } else {
                false
            }
        }
        _ => false,
    };


    let call_expr = if is_async {
        quote! { #fn_name(#(#call_args),*).await }
    } else {
        quote! { #fn_name(#(#call_args),*) }
    };

    let result_handling = if returns_result {
        quote! {
            match result {
                Ok(v) => {
                    #[allow(unused_imports)]
                    use crate::qt::dispatcher::{ViaSerialize, Wrap};
                    Wrap(v).into_dispatch_result()
                }
                Err(e) => Err(e.to_string()),
            }
        }
    } else {
        quote! {
            {
                #[allow(unused_imports)]
                use crate::qt::dispatcher::{ViaSerialize, Wrap};
                Wrap(result).into_dispatch_result()
            }
        }
    };

    let expanded = quote! {
        #original

        #[allow(non_snake_case)]
        #vis async fn #wrapper_name(args: serde_json::Value)
            -> Result<serde_json::Value, String>
        {
            let obj = args
                .as_object()
                .ok_or("args must be JSON object")?;

            let expected_keys = vec![
                #(stringify!(#arg_names)),*
            ];

            for key in obj.keys() {
                if !expected_keys.contains(&key.as_str()) {
                    return Err(format!("unexpected argument: {}", key));
                }
            }

            for expected in &expected_keys {
                if !obj.contains_key(*expected) {
                    return Err(format!("missing argument: {}", expected));
                }
            }

            #(#let_stmts)*

            let result = #call_expr;

            #result_handling
        }
    };

    expanded.into()
}

struct HandlerList {
    paths: Punctuated<Path, Token![,]>,
}

impl Parse for HandlerList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(HandlerList {
            paths: Punctuated::parse_terminated(input)?,
        })
    }
}

#[proc_macro]
pub fn generate_handler(input: TokenStream) -> TokenStream {
    let HandlerList { paths } = parse_macro_input!(input as HandlerList);

    let arms = paths.iter().map(|path| {
        let last_ident = &path.segments.last().expect("path must have at least one segment").ident;
        let name_str = last_ident.to_string();

        let wrapper_ident = syn::Ident::new(&format!("__cmd__{}", last_ident), last_ident.span());
        let mut wrapper_path = path.clone();
        wrapper_path.segments.last_mut().unwrap().ident = wrapper_ident;

        quote! {
            #name_str => #wrapper_path(args).await,
        }
    });

    let expanded = quote! {
        pub async fn dispatch_command(
            name: &str,
            args: serde_json::Value
        ) -> Result<serde_json::Value, String> {
            match name {
                #(#arms)*
                _ => Err(format!("Unknown command: {}", name)),
            }
        }
    };

    expanded.into()
}