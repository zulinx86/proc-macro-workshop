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

    let checks = fields_ident.iter().map(|ident| {
        let err_msg = format!("{} is required.", ident);
        quote! {
            if self.#ident.is_none() {
                return Err(#err_msg.into())
            }
        }
    });

    let expanded = quote! {
        #vis struct #builder_ident {
            #(#fields_ident: Option<#fields_ty>),*
        }

        impl #builder_ident {
            #(pub fn #fields_ident(&mut self, #fields_ident: #fields_ty) -> &mut Self {
                self.#fields_ident = Some(#fields_ident);
                self
            })*

            pub fn build(&mut self) -> Result<#ident, Box<dyn std::error::Error>> {
                #(#checks)*

                Ok(#ident {
                    #(#fields_ident: self.#fields_ident.clone().unwrap()),*
                })
            }
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
