use deluxe::ExtractAttributes;
use proc_macro::TokenStream as TS1;
use proc_macro2::{TokenStream as TS2};
use quote::format_ident;
use syn::{DeriveInput, LitBool, LitStr, Type};

#[derive(Default, Debug, ExtractAttributes)]
#[deluxe(attributes(encryption))]
struct EncryptionAttrs {
    sanitize: Option<LitStr>,
    errors: Option<Type>,
    skip: Option<LitBool>
}

// Start of derive and field attribute derives
#[proc_macro_derive(Encryption, attributes(encryption))]
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
    let node_form = format_ident!("{}Form", node);
    let node_error = format_ident!("{}Error", node);

    // Create encoding error
    let error = format!("Unable to parse {} jsonb object", node);

    // All column attributed information
    let mut all_column_fields = vec![];
    let mut all_column_inner_types = vec![];
    let mut all_form_struct_fields = vec![];
    let mut all_error_struct_fields = vec![];

    let mut all_form_props = vec![];
    let mut sanitizers = vec![];

    let mut all_attributed_fields = vec![];
    let mut all_attributed_inner_types = vec![];

    // Loop through all fields
    for (
        field,
        ty,
        is_attributed,
        attrs
    ) in
        derive_utils::derive_all_fields::<&str, EncryptionAttrs>(&ast, "encryption")
    {
        // Retrieve inner type
        let inner_ty = derive_utils::derive_parse_inner_type(&ty);
        let error_type = attrs.errors.clone()
            .unwrap_or(ty.clone());

        // Include all column fields
        all_column_fields.push(field.clone());
        all_column_inner_types.push(inner_ty.clone());

        // Check all attributed fields
        let is_skipped = if let Some(b) = attrs.skip.clone() {
            b.value()
        } else {
            false
        };

        if is_attributed && !is_skipped {
            all_attributed_fields.push(field.clone());
            all_attributed_inner_types.push(inner_ty.clone());
        }

        // Create form fields
        all_form_struct_fields.push(quote::quote!{
            #[serde(skip_serializing_if = "Null::undefined")]
            pub #field: #ty
        });

        all_form_props.push(quote::quote! {
            pub fn #field(&self) -> #inner_ty {
                self.#field.clone().take().unwrap_or_default()
            }
        });

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
                _ => {}
            }
        }

        // Create error fields
        all_error_struct_fields.push(quote::quote!{
            #[serde(skip_serializing_if = "Null::undefined")]
            pub #field: #error_type
        });
    }

    // Cipher Related
    //________________________________________________________
    token.extend(quote::quote! {
        #(
            pub fn #all_attributed_fields() -> #all_attributed_inner_types {
                crate::clone().#all_attributed_fields.take().unwrap_or_default()
            }
        )*

        impl #node {
             /// Converts the current instance to another type `T` that implements `From<Self>`.
             ///
             /// # Returns
             /// - An instance of type `T`, created from the current instance.
            pub fn to<T: From<Self>>(&self) -> T {
                T::from(self.clone())
            }

            /// Updates the current instance with the values from another instance of the same type.
            ///
            /// # Parameters
            /// - `form`: A reference to another instance of `Self` whose values will be copied.
            ///
            /// # Returns
            /// - A mutable reference to the updated instance (`self`).
            pub fn mutate(&mut self, form: &Self) -> &mut Self {
                #(
                    self.#all_column_fields = form.#all_column_fields.clone();
                )*

                self
            }

            /// Encrypts sensitive fields of the current instance using the `CipherExt` trait.
            ///
            /// # Returns
            /// - A new instance of `Self` with encrypted fields.
            pub fn encrypt(&self) -> Self {
                use ciphers::CipherExt;

                let mut data = self.clone();

                #(
                    data.#all_attributed_fields = data.#all_attributed_fields.encrypt();
                )*

                data
            }

            /// Decrypts sensitive fields of the current instance using the `CipherExt` trait.
            ///
            /// # Returns
            /// - A new instance of `Self` with decrypted fields.
            pub fn decrypt(&self) -> Self {
                use ciphers::CipherExt;

                let mut data = self.clone();

                #(
                    data.#all_attributed_fields = data.#all_attributed_fields.decrypt();
                )*

                data
            }

            /// Checks if the current instance is equivalent to the default value of its type.
            ///
            /// # Returns
            /// - `true` if the instance is equal to the default value.
            /// - `false` otherwise.
             pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            #(
                pub fn #all_column_fields(&self) -> #all_column_inner_types {
                    self.clone().#all_column_fields.take().unwrap_or_default()
                }
            )*
        }

        impl actix_web::Responder for #node {
            type Body = actix_web::body::BoxBody;

            fn respond_to(self, _req: &actix_web::HttpRequest) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(serde_json::json!({
                    "code": 200,
                    "data": self
                }))
            }
        }

        pub mod parsers {
            use sqlx::Row;
            use crate::#node;

            /// Parses a PostgreSQL row (`PgRow`) into an instance of `Self`.
            ///
            /// # Parameters
            /// - `row`: A reference to a `PgRow` containing the data to be parsed.
            ///
            /// # Returns
            /// - An instance of `Self` populated with the values from the `PgRow`.
            ///   If a field cannot be retrieved, it will use the `Null` type as a fallback.
            pub fn parse<T>(value: &sqlx::postgres::PgRow, column: T) -> responder::Result<#node>
            where
                T: ToString
            {
                if let Ok(d) = value.try_get::<#node, &str>(&column.to_string()) {
                    return Ok(d.decrypt());
                }

                Err(responder::to(#error))
            }

            /// Converts a SQLx query result into a `responder::Result<Self>`.
            ///
            /// # Parameters
            /// - `row`: A `Result` containing a `PgRow` or an error.
            ///
            /// # Returns
            /// - `Ok(Self)` if the row is successfully parsed and is not empty.
            /// - `Err(responder::to(#error))` if the row is empty or the query fails.
            pub fn result<T>(value: sqlx::Result<sqlx::postgres::PgRow>, column: T) -> responder::Result<#node>
            where
                T: ToString
            {
                if let Ok(d) = value {
                    return parse(&d, column);
                }

                Err(responder::to(#error))
            }
        }

        impl sqlx::Type<sqlx::Postgres> for #node {
            fn type_info() -> sqlx::postgres::PgTypeInfo {
                <sqlx::types::Json<Self> as sqlx::Type<sqlx::Postgres>>::type_info()
            }
        }

        impl<'q> sqlx::Encode<'q, sqlx::Postgres> for #node {
            fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> Result<sqlx::encode::IsNull, Box<dyn serde::ser::StdError + Send + Sync + 'static>> {
                <sqlx::types::Json<&Self> as sqlx::Encode<'q, sqlx::Postgres>>::encode(sqlx::types::Json(self), buf)
            }
        }

        impl<'r> sqlx::Decode<'r, sqlx::Postgres> for #node {
            fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
                let bytes = value.as_str()?
                    .strip_prefix('\u{1}')
                    .unwrap_or(value.as_str()?);

                Ok(serde_json::from_str(bytes)?)
            }
        }
    });

    // Form Related
    //________________________________________________________
    token.extend(quote::quote! {
        #[derive(Debug, Clone, Default, PartialEq)]
        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #node_form {
            #(#all_form_struct_fields,)*
        }

        impl #node_form {
            /// Checks if the current instance is equivalent to the default value of its type.
            ///
            /// # Returns
            /// - `true` if the instance is equal to the default value.
            /// - `false` otherwise.
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

             /// Converts the current instance to another type `T` that implements `From<Self>`.
             ///
             /// # Returns
             /// - An instance of type `T`, created from the current instance.
            pub fn to<T: From<Self>>(&self) -> T {
                T::from(self.clone())
            }

            /// Sanitizes the current instance by applying a series of sanitizer functions.
            ///
            /// # Returns
            /// - A sanitized copy of the current instance.
            ///
            /// # Implementation
            /// - Each sanitizer in the `#sanitizers` sequence is applied to the cloned instance.
            pub fn sanitize(&self) -> Self {
                let mut data = self.clone();

                #(#sanitizers)*

                data
            }

            #(#all_form_props)*
        }

        impl From<#node> for #node_form {
            fn from(value: #node) -> Self {
                let mut data = Self::default();

                #(
                    data.#all_column_fields = value.#all_column_fields.clone();
                )*

                data
            }
        }

        impl From<#node_form> for #node {
            fn from(value: #node_form) -> Self {
                let mut data = Self::default();

                #(
                    data.#all_column_fields = value.#all_column_fields.clone();
                )*

                data
            }
        }
    });

    // Error Related
    // ________________________________________________________
    token.extend(quote::quote! {
        #[derive(Debug, Clone, Default, PartialEq)]
        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #node_error {
            #(#all_error_struct_fields,)*
        }

       impl #node_error {
            /// Checks if the current instance is equivalent to the default value of its type.
            ///
            /// # Returns
            /// - `true` if the instance is equal to the default value of `Self`.
            /// - `false` otherwise.
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            /// Validates the current instance.
            ///
            /// This method checks whether the instance is empty (equivalent to the default value).
            /// If it is empty, the method returns `Ok(())`. Otherwise, it returns an error.
            ///
            /// # Returns
            /// - `Ok(())` if the instance is empty (i.e., equal to the default value).
            /// - `Err(responder::to(self))` if the instance is not empty, returning an error based on `self`.
            pub fn validate(&self) -> responder::Result<()> {
                if self.is_empty() {
                    return Ok(())
                }

                Err(responder::to(self))
            }
        }

        impl #node_form {
            /// Converts the current instance to the associated error type.
            ///
            /// # Returns
            /// - A default instance of Error
            pub fn to_error(&self) -> #node_error {
                #node_error::default()
            }
        }
    });

    // Return the new token
    Ok(token)
}