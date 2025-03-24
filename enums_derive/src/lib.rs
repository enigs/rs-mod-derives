use proc_macro::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput, Ident, Lit, Meta, MetaNameValue, Variant};

#[proc_macro_derive(Enums)]
pub fn derive_enum_iter(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let ident = ast.ident;
    let variants = match ast.data {
        Data::Enum(ref data) => &data.variants,
        _ => panic!("Enums can only be derived for enums"),
    };

    // Find default variant
    let default_variant = variants.iter().find(|variant| {
        variant.attrs.iter().any(|attr| attr.path().is_ident("default"))
    });

    // If no default found, use first variant
    let default_variant = match default_variant {
        Some(v) => v,
        None => &variants[0],
    };
    
    let default_variant_ident = &default_variant.ident;

    // Collect variants and their rename values
    let variants: Vec<(Ident, String, String)> = variants
        .iter()
        .map(|variant| {
            let variant_ident = variant.ident.clone();
            let rename_value = extract_rename_value(variant);
            (variant_ident, rename_value.clone(), rename_value.to_lowercase())
        })
        .collect();

    let mut variant_ident = vec![];
    let mut variant_string = vec![];
    let mut variant_lowered = vec![];

    for (v, s, l) in variants {
        variant_ident.push(v);
        variant_string.push(s);
        variant_lowered.push(l);
    }


    let token = quote::quote!{
        impl std::fmt::Display for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                let text = match self {
                    #(Self::#variant_ident => #variant_string,)*
                };

                write!(f, "{}", text)
            }
        }

        impl<'de> serde::de::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let variant = String::deserialize(deserializer)?;

                match variant.to_lowercase().as_str() {
                    #(#variant_lowered => Ok(Self::#variant_ident),)*
                    _ => Err(serde::de::Error::unknown_variant(
                        &variant,
                        &[
                            #(#variant_string,)*
                        ]
                    )),
                }
            }
        }

        impl serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
                let variant_str = match self {
                     #(Self::#variant_ident => #variant_string,)*
                };

                serializer.serialize_str(variant_str)
            }
        }

        impl From<String> for #ident {
            fn from(value: String) -> Self {
                match value.to_lowercase().as_str() {
                    #(#variant_lowered => Self::#variant_ident,)*
                    
                    _ => Self::#default_variant_ident,
                }
            }
        }

        impl From<&String> for #ident {
            fn from(value: &String) -> Self {
                Self::from(value.clone())
            }
        }
        
        impl From<&str> for #ident {
            fn from(value: &str) -> Self {
                Self::from(value.to_string())
            }
        }
        
        impl From<Option<String>> for #ident {
            fn from(value: Option<String>) -> Self {
                Self::from(value.unwrap_or_default())
            }
        }

        impl From<&Option<String>> for #ident {
            fn from(value: &Option<String>) -> Self {
                Self::from(value.clone().unwrap_or_default())
            }
        }
        
        impl From<Option<&str>> for #ident {
            fn from(value: Option<&str>) -> Self {
                Self::from(value.unwrap_or_default())
            }
        }
        
        impl From<libs::nulls::Null<String>> for #ident {
            fn from(value: libs::nulls::Null<String>) -> Self {
                Self::from(value.take().unwrap_or_default())
            }
        }
        
        impl From<libs::nulls::Null<&str>> for #ident {
            fn from(value: libs::nulls::Null<&str>) -> Self {
                Self::from(value.take().unwrap_or_default())
            }
        }

    };

    token.into()
}



fn extract_rename_value(variant: &Variant) -> String {
    for attr in &variant.attrs {
        if attr.path().is_ident("sqlx") {
            if let Ok(Meta::NameValue(MetaNameValue {
              value: syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }),
              ..
              })) = attr.parse_args::<Meta>() {
                return lit_str.value();
            }
        }
    }

    // Fallback to variant name if no rename found
    variant.ident.to_string()
}