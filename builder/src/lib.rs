use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type, Visibility};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate(input) {
        Ok(generated) => generated,
        Err(err) => err.to_compile_error().into(),
    }
}

struct ParseResult {
    ident: Ident,
    vis: Visibility,
    fields: Vec<(Ident, Type)>,
}

fn parse(derive_input: DeriveInput) -> Result<ParseResult, syn::Error> {
    // Parse input
    let ident = derive_input.ident;
    let vis = derive_input.vis;
    let fields: Vec<(Ident, Type)> = match derive_input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields
                .named
                .into_iter()
                .map(|field| {
                    // SAFETY: `fields` is a `FieldsNamed` object.
                    let ident = field.ident.unwrap();
                    let ty = field.ty;
                    (ident, ty)
                })
                .collect(),
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

    Ok(ParseResult { ident, vis, fields })
}

fn apply_fields<F>(fields: &[(Ident, Type)], f: F) -> Vec<TokenStream>
where
    F: Fn(&(Ident, Type)) -> TokenStream,
{
    fields.iter().map(f).collect()
}

fn builder_declare(
    builder_ident: &Ident,
    vis: &Visibility,
    fields: &[(Ident, Type)],
) -> TokenStream {
    let builder_fields = apply_fields(fields, |(ident, ty)| {
        quote! {
            #ident: Option<#ty>
        }
    });

    quote! {
        #vis struct #builder_ident {
            #(#builder_fields),*
        }
    }
}

fn builder_setters(fields: &[(Ident, Type)]) -> TokenStream {
    let setters = apply_fields(fields, |(ident, ty)| {
        quote! {
            pub fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = Some(#ident);
                self
            }
        }
    });

    quote! {
        #(#setters)*
    }
}

fn builder_build_checkers(fields: &[(Ident, Type)]) -> TokenStream {
    let checkers = apply_fields(fields, |(ident, _)| {
        let err_msg = format!("{} is required.", ident);
        quote! {
            if self.#ident.is_none() {
                return Err(#err_msg.into())
            }
        }
    });

    quote! {
        #(#checkers)*
    }
}

fn builder_build_ret(ident: &Ident, fields: &[(Ident, Type)]) -> TokenStream {
    let items = apply_fields(fields, |(ident, _)| {
        quote! {
            #ident: self.#ident.clone().unwrap()
        }
    });

    quote! {
        Ok(#ident {
            #(#items),*
        })
    }
}

fn builder_build(ident: &Ident, fields: &[(Ident, Type)]) -> TokenStream {
    let builder_build_checkers = builder_build_checkers(fields);
    let builder_build_ret = builder_build_ret(ident, fields);

    quote! {
        pub fn build(&mut self) -> Result<#ident, Box<dyn std::error::Error>> {
            #builder_build_checkers
            #builder_build_ret
        }
    }
}

fn init_builder(ident: &Ident, builder_ident: &Ident, fields: &[(Ident, Type)]) -> TokenStream {
    let items = apply_fields(fields, |(ident, _)| {
        quote! {
            #ident: None
        }
    });

    quote! {
        impl #ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#items),*
                }
            }
        }
    }
}

fn generate(derive_input: DeriveInput) -> Result<proc_macro::TokenStream, syn::Error> {
    let ParseResult { ident, vis, fields } = parse(derive_input)?;

    let builder_ident = format_ident!("{}Builder", ident);
    let builder_declare = builder_declare(&builder_ident, &vis, &fields);
    let builder_setters = builder_setters(&fields);
    let builder_build = builder_build(&ident, &fields);
    let init_builder = init_builder(&ident, &builder_ident, &fields);

    let expanded = quote! {
        #builder_declare

        impl #builder_ident {
            #builder_setters
            #builder_build
        }

        #init_builder
    };

    Ok(proc_macro::TokenStream::from(expanded))
}
