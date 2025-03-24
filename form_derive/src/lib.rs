use deluxe::ExtractAttributes;
use proc_macro::TokenStream as TS1;
use proc_macro2::{Ident, TokenStream as TS2};
use quote::format_ident;
use syn::{DeriveInput, LitBool, LitStr, Type};

// Set ReferenceAttrs
#[derive(Default, Debug, ExtractAttributes)]
#[deluxe(attributes(reference))]
struct ReferenceAttrs {
    pub model: Option<Ident>
}

// Set FormAttrs struct
#[derive(Default, Debug, deluxe::ExtractAttributes)]
#[deluxe(attributes(form))]
struct FormAttrs {
    pub sanitize: Option<LitStr>,
    pub error: Option<Type>,
    pub skip_refs: Option<LitBool>
}

// Start of derive and field attribute derives
#[proc_macro_derive(Form, attributes(form, reference))]
pub fn main(stream: proc_macro::TokenStream) -> TS1 {
    derive(stream.into()).unwrap().into()
}

// Start of derive and token processing
fn derive(stream: TS2) -> deluxe::Result<TS2> {
    // Parse token stream
    let ast: DeriveInput = syn::parse2(stream)?;
    let node = &ast.ident.clone();

    // Create main token stream
    let mut token = quote::quote!{};
    let reference_attrs = derive_utils::derive_struct_attrs::<ReferenceAttrs>(&ast);

    // Create error & response node
    let node_error = format_ident!("{}Error", node.to_string().replace("Form", ""));

    let mut sanitizers = vec![];
    let mut fields = vec![];
    let mut ref_fields = vec![];
    let mut error_derives = vec![];
    let mut error_fields = vec![];
    let mut error_types = vec![];
    let mut cloned_fields = vec![];
    let mut all_props = vec![];

    // Loop through all fields
    for (
        field,
        ty,
        _is_attributed,
        attrs
    ) in
        derive_utils::derive_all_fields::<&str, FormAttrs>(&ast, "form")
    {
        // Set type string
        let ty_to_str = derive_utils::derive_type_to_string(&ty);
        let inner_ty = derive_utils::derive_parse_inner_type(&ty);
        if ty_to_str.starts_with("Null") {
            all_props.push(quote::quote! {
                pub fn #field(&self) -> Option<#inner_ty> {
                    self.#field.clone().take()
                }
            });
        }

        // Push into field vec
        fields.push(field.clone());

        // Check if current field should be skipped
        if !(attrs.skip_refs.is_some() && attrs.skip_refs.clone().unwrap().value) {
            ref_fields.push(field.clone());
        }

        // Set sanitizers
        if let Some(attr) = attrs.sanitize {
            match attr.value().as_str() {
                "lowercase" => sanitizers.push(quote::quote! {
                            if let Null::Value(value) = data.#field.clone() {
                                if !value.is_empty() {
                                    data.#field = Null::Value(value.to_string().trim().to_lowercase().to_string());
                                }
                            }
                        }),
                "normalize_name" => sanitizers.push(quote::quote! {
                            if let Null::Value(value) = data.#field.clone() {
                                let value = value.trim();

                                if !value.is_empty() {
                                    data.#field = Null::Value(title_case::title_case(&value, "Jr Sr I II III IV V VI VII VIII IX X XX XXX De Los DeLos"));
                                }
                            }
                        }),
                "trim" => sanitizers.push(quote::quote! {
                            if let Null::Value(value) = data.#field.clone() {
                                if !value.is_empty() {
                                    data.#field = Null::Value(value.to_string().trim().to_string());
                                }
                            }
                        }),
                "trim_slash" => sanitizers.push(quote::quote! {
                            if let Null::Value(value) = data.#field.clone() {
                                if !value.is_empty() {
                                    data.#field = Null::Value(value
                                        .to_string()
                                        .trim()
                                        .trim_end_matches('/')
                                        .trim()
                                        .to_string());
                                }
                            }
                        }),
                "dedup" => sanitizers.push(quote::quote! {
                            if let Null::Value(value) = data.#field.clone() {
                                if !value.is_empty() {
                                    let mut items = value.clone();
                                    items.dedup();

                                    data.#field = Null::Value(items);
                                }
                            }
                        }),
                _ => {}
            }
        }

        // Set errors
        error_fields.push(field.clone());
        error_types.push(match () {
            _ if attrs.error.is_some() => attrs.error.unwrap(),
            _ => ty.clone()
        });

        error_derives.push(quote::quote! {
            #[serde(skip_serializing_if = "Null::undefined")]
        });

        let cloned_field = format_ident!("clone_{}", field);
        cloned_fields.push(quote::quote!{
            pub fn #cloned_field(&self, value: &#ty) -> Self {
                let mut data = self.clone();

                data.#field = value.clone();

                data
            }
        });
    }

    // Extend functionality
    token.extend(quote::quote! {
        impl #node {
            /// Checks if the current instance is equivalent to the default value of its type.
            ///
            /// # Returns
            /// - `true` if the instance is equal to the default value.
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            /// Converts the current instance to another type `T` that implements `From<Self>`.
            ///
            /// # Returns
            /// An instance of type `T`.
            pub fn to<T: From<Self>>(&self) -> T {
                T::from(self.clone())
            }

            /// Converts the current instance to the associated error type `Self::Error`.
            ///
            /// # Returns
            /// A default instance of `Self::Error`.
            pub fn to_error(&self) -> #node_error {
                #node_error::default()
            }

            /// Converts the current instance to a JSON representation (`sqlx::types::Json<Self>`).
            ///
            /// # Returns
            /// A JSON representation of the current instance.
            pub fn to_json(&self) -> sqlx::types::Json<Self> {
                sqlx::types::Json::from(self.clone())
            }

            /// Sanitizes the current instance by applying a series of sanitizer functions.
            ///
            /// # Returns
            /// A sanitized copy of the current instance.
            pub fn sanitize(&self) -> Self {
                let mut data = self.clone();

                #(#sanitizers)*

                data
            }

            #(#all_props)*

            #(#cloned_fields)*
        }

        #[derive(Debug, Clone, Default, PartialEq)]
        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #node_error {
            #(
                #error_derives
                pub #error_fields: #error_types,
            )*
        }

        // Error implementations
        impl #node_error {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            pub fn validate(&self) -> libs::responder::Result<()> {
                if self.is_empty() {
                    return Ok(())
                }

                Err(libs::responder::to(self))
            }
        }

        impl actix_web::Responder for #node_error {
            type Body = actix_web::body::BoxBody;

            fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(self)
            }
        }
    });

    // Check if reference exists
    if let Some(refs) = reference_attrs.model {
        token.extend(quote::quote! {
            impl From<#node> for #refs {
                fn from(value: #node) -> Self {
                    let mut data = Self::default();

                    #(
                        data.#ref_fields = value.#ref_fields.clone();
                    )*

                    data
                }
            }

            impl From<#refs> for #node {
                fn from(value: #refs) -> Self {
                    let mut data = Self::default();

                    #(
                        data.#ref_fields = value.#ref_fields.clone();
                    )*

                    data
                }
            }
        });
    }

    // Return the new token
    Ok(token)
}