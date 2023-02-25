use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate_builder(input) {
        Ok(generated) => generated,
        Err(err) => err.to_compile_error().into(),
    }
}

fn generate_builder(derive_input: DeriveInput) -> Result<TokenStream, syn::Error> {
    let ident = derive_input.ident;
    let vis = derive_input.vis;
    let builder_ident = format_ident!("{}Builder", ident);

    let (fields_ident, fields_ty): (Vec<_>, Vec<_>) = match derive_input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields
                .named
                .into_iter()
                .map(|field| {
                    // SAFETY: Named fields.
                    let ident = field.ident.unwrap();
                    let ty = field.ty;
                    (ident, ty)
                })
                .unzip(),
            _ => {
                return Err(syn::Error::new_spanned(
                    ident,
                    "Only named fields are allowed.",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                ident,
                "Only struct type is allowed.",
            ));
        }
    };

    let expanded = quote! {
        #vis struct #builder_ident {
            #(#fields_ident: Option<#fields_ty>),*
        }

        impl #ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#fields_ident: None),*
                }
            }
        }
    };

    Ok(TokenStream::from(expanded))
}
