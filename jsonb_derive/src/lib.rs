use proc_macro::TokenStream;
use syn::DeriveInput;

// Entry point for our macro
#[proc_macro_derive(Jsonb)]
pub fn main(stream: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(stream).unwrap();
    let node = ast.ident;
    let error = format!("Unable to parse {} jsonb object", node);

    TokenStream::from(quote::quote! {
        impl #node {
            /// Checks if the current instance is equivalent to the default value of its type.
            ///
            /// # Returns
            /// - `bool` - `true` if `self` is equal to the default value, otherwise `false`.
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }
        }

        pub mod parsers {
            use sqlx::Row;
            use crate::#node;

            /// Extracts a value of type `Self` from the specified column in the given PostgreSQL row.
            ///
            /// # Parameters
            /// - `value`: A reference to the PostgreSQL row (`PgRow`) from which to extract the value.
            /// - `column`: The name of the column to extract the value from. It must implement `ToString`.
            ///
            /// # Returns
            /// - `Ok(Self)` if the value is successfully extracted.
            /// - `Err(responder::to(Self))` if the value cannot be extracted.
            pub fn row<T>(value: &sqlx::postgres::PgRow, column: T) -> responder::Result<#node>
            where
                T: ToString
            {
                if let Ok(d) = value.try_get::<#node, &str>(&column.to_string()) {
                    return Ok(d);
                }

                Err(responder::to(#error))
            }

            /// Extracts a value of type `#node` from the specified column in a SQLx result containing a PostgreSQL row.
            ///
            /// # Parameters
            /// - `value`: A SQLx result containing a PostgreSQL row (`PgRow`).
            /// - `column`: The name of the column to extract the value from. It must implement `ToString`.
            ///
            /// # Returns
            /// - `Ok(Self)` if the row exists and the value is successfully extracted.
            /// - `Err(responder::to(Self))` if the row does not exist or the value cannot be extracted.
            pub fn result<T>(value: sqlx::Result<sqlx::postgres::PgRow>, column: T) -> responder::Result<#node>
            where
                T: ToString
            {
                if let Ok(d) = value {
                    return row(&d, column);
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
    })
}