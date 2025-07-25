
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(ToYaml)]
pub fn derive_to_yaml(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let fields = match input.data {
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(named) => named.named,
            _ => panic!("ToYaml only supports structs with named fields"),
        },
        _ => panic!("ToYaml can only be derived for structs"),
    };

    let field_names = fields.iter().map(|f| f.ident.as_ref().unwrap());
    let field_keys = field_names.clone().map(|f| f.to_string());

    let expanded = quote! {
        impl ToYaml for #name {
            fn to_yaml(&self) -> yaml_rust2::Yaml {
                let mut map = hashlink::LinkedHashMap::new();
                #(
                    map.insert(
                        yaml_rust2::Yaml::String(#field_keys.into()),
                        self.#field_names.to_yaml()
                    );
                )*
                yaml_rust2::Yaml::Hash(map)
            }
        }
    };

    expanded.into()
}