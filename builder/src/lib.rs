use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, GenericArgument};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let builder_name = Ident::new(&(name.to_string() + "Builder"), Span::call_site());

    let mut builder_fields = vec![];
    let mut builder_fields_inits = vec![];
    let mut setters = vec![];
    let mut build_items = vec![];

    if let Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = input.data
    {
        for field in named {
            let syn::Field { ty, ident, .. } = field;

            let mut optional_value = None;
            let is_option = if let syn::Type::Path(ty) = ty.clone() {
                let segments = ty.path.segments.first().unwrap();
                let option_ident = segments.ident == "Option";

                let generic_arg =
                    if let syn::PathArguments::AngleBracketed(v) = segments.arguments.clone() {
                        if let Some(GenericArgument::Type(ty)) = v.args.first() {
                            optional_value = Some(ty.to_owned());
                            true
                        } else {
                            false
                        }
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
            let arg_type = if is_option { optional_value } else { Some(ty) };

            let setter = quote! {
                fn #ident (&mut self, #ident: #arg_type ) -> &mut Self {
                    self. #ident = Some(#ident);
                    self
                }
            };

            let build_item = if !is_option {
                quote!(
                    #ident: self.#ident.to_owned().ok_or(concat!("Error: ", stringify!(#ident), " was not initialized"))?
                )
            } else {
                quote!(
                    #ident: self.#ident.to_owned()
                )
            };
            build_items.push(build_item);

            builder_fields.push(builder_field);
            builder_fields_inits.push(builder_field_init);
            setters.push(setter);
        }
    } else {
        unimplemented!()
    }

    let nothing = quote!(
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
                Ok(#name {
                    #(#build_items),*
                })
            }
        }
    );
    proc_macro::TokenStream::from(nothing)
}
