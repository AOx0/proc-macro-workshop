use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, GenericArgument};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let builder_name = Ident::new(&(name.to_string() + "Builder"), Span::call_site());
    let builder_err_name = Ident::new(
        &("Error".to_string() + &name.to_string() + "Builder"),
        Span::call_site(),
    );

    let mut builder_fields = vec![];
    let mut builder_fields_inits = vec![];
    let mut setters = vec![];
    let mut set_checks = vec![];
    let mut build_items = vec![];

    if let Data::Struct(members) = input.data {
        let fields = members.fields;

        if let Fields::Named(fields) = fields {
            for field in fields.named {
                let syn::Field { ty, ident, .. } = field;

                let is_option = if let syn::Type::Path(ty) = ty.clone() {
                    let segments = ty.path.segments;
                    let segments = segments.first().unwrap();
                    let option_ident = segments.ident == "Option";

                    let generic_arg =
                        if let syn::PathArguments::AngleBracketed(v) = segments.arguments.clone() {
                            matches!(v.args.first(), Some(GenericArgument::Type(_)))
                        } else {
                            false
                        };

                    option_ident && generic_arg
                } else {
                    false
                };

                let builder_field = if !is_option {
                    quote!(pub #ident : Option< #ty >)
                } else {
                    quote!(pub #ident : #ty)
                };

                let builder_field_init = quote!(#ident: None);
                let setter = quote! {fn #ident (&mut self, #ident: #ty ) -> &mut Self {
                    self. #ident = Some(#ident);
                    self
                } };

                let err_msg = format!("Error: {} wasn't initialized", quote! {#ident});
                let set_check = if !is_option {
                    quote!(
                        if self. #ident .is_none() {
                            return Err(
                                Box::new(
                                    #builder_err_name (& #err_msg )
                                )
                            );
                        }
                    )
                } else {
                    quote!()
                };
                set_checks.push(set_check);

                let build_item = if !is_option {
                    quote!(
                        #ident: self.#ident.to_owned().unwrap()
                    )
                } else {
                    quote!(
                        #ident: self.#ident.to_owned().unwrap_or(None)
                    )
                };
                build_items.push(build_item);

                builder_fields.push(builder_field);
                builder_fields_inits.push(builder_field_init);
                setters.push(setter);
            }
        }
    }

    let nothing = quote!(
        #[derive(Debug)]
        struct #builder_err_name <'a>(&'a str);

        impl<'a> std::error::Error for #builder_err_name <'a> {}

        impl<'a> core::fmt::Display for #builder_err_name <'a> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl<'a> From<&'a str> for #builder_err_name<'a> {
            fn from(arg: &'a str) -> Self {
                #builder_err_name (arg)
            }
        }

        impl #name {
            fn builder() -> #builder_name {
                #builder_name {
                    #(#builder_fields_inits),*
                }
            }
        }

        struct #builder_name {
            #(#builder_fields),*
        }

        impl  #builder_name {
            #(#setters)*
        }

        impl  #builder_name {
            pub fn build(&mut self) -> Result<#name, Box<dyn std::error::Error>> {
                #(#set_checks)*

                Ok(#name {
                    #(#build_items),*
                })
            }
        }
    );
    proc_macro::TokenStream::from(nothing)
}
