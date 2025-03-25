use deluxe::ExtractAttributes;
use proc_macro::TokenStream as TS1;
use proc_macro2::{Ident, TokenStream as TS2};
use quote::format_ident;
use std::collections::HashMap;
use syn::{DeriveInput, LitStr, Type};

// Table attribute
#[derive(Default, Debug, ExtractAttributes)]
#[deluxe(attributes(table))]
struct TableAttrs {
    alias: Option<LitStr>,
    rename: Option<LitStr>,
}

// Column attribute
#[derive(Default, Debug, ExtractAttributes)]
#[deluxe(attributes(column))]  // Fixed typo: columnn -> column
struct ColumnAttrs {}

// Start of derive and field attribute derives
#[proc_macro_derive(PostgreSQL, attributes(table, column))]
pub fn main(stream: TS1) -> TS1 {
    derive(stream.into()).unwrap().into()
}

// Start of derive and token processing
fn derive(stream: TS2) -> deluxe::Result<TS2> {
    // Parse token stream
    let ast: DeriveInput = syn::parse2(stream)?;
    let node = &ast.ident.clone();

    // Create main token stream
    let mut token = quote::quote!{};
    let table_attrs = derive_utils::derive_struct_attrs::<TableAttrs>(&ast);

    // Create table name
    let table_name = derive_utils::derive_snake_case(table_attrs.rename
        .map(|s| s.value())
        .unwrap_or(node.to_string()));

    let aliases = if let Some(alias) = table_attrs.alias {
        alias.value()
            .replace(" ", "")
            .replace(",", ";")
            .to_lowercase()
            .split(';')
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
    } else {
        Vec::<String>::new()  // Specify type explicitly
    };

    // Create error message
    let error = format!("No matching record(s) found in {} table", table_name);

    // All column attributed information
    let mut all_props = Vec::<TS2>::new();  // Specify types explicitly
    let mut all_setters = Vec::<TS2>::new();
    let mut all_setter_opts = Vec::<TS2>::new();
    let mut all_clones = Vec::<TS2>::new();
    let mut all_clears = Vec::<TS2>::new();
    let mut all_cleable_fields = Vec::<Ident>::new();
    let mut all_update_fields = Vec::<Ident>::new();
    let mut all_update_columns = Vec::<String>::new();

    let mut all_attributed_fields = Vec::<Ident>::new();
    let mut all_attributed_inner_ty = Vec::<Type>::new();
    let mut all_attributed_renamed = Vec::<String>::new();

    // Set text values
    let mut all_const_names = Vec::<Ident>::new();
    let mut all_aliased = Vec::<String>::new();
    let mut all_renamed = Vec::<String>::new();
    let mut all_plain = Vec::<String>::new();
    let mut all_tabled = Vec::<String>::new();

    let mut map_sub_parser:HashMap<Ident, Vec<(Ident, Type, String)>> = HashMap::new();
    let mut map_sub_alias:HashMap<Ident, Vec<String>> = HashMap::new();

    // Loop through all fields
    for (
        field,
        ty,
        is_attributed,
        _attrs
    ) in
        derive_utils::derive_all_fields::<&str, ColumnAttrs>(&ast, "column")
    {
        let ty_to_str = derive_utils::derive_type_to_string(&ty);
        let inner_ty = derive_utils::derive_parse_inner_type(&ty);
        // let inner_ty_to_str = derive_utils::derive_type_to_string(&inner_ty);

        // Set all update fields
        if field.to_string().as_str() != "id" && is_attributed {
            all_update_fields.push(field.clone());
            all_update_columns.push(format!("{} = ${{}}", field.clone()));
        }

        // Create props
        match ty_to_str.to_lowercase().starts_with("null<") {
            true => all_props.push(quote::quote! {
                pub fn #field(&self) -> Option<#inner_ty> {
                    self.#field.clone().take()
                }
            }),
            false => all_props.push(quote::quote! {
                pub fn #field(&self) -> #ty {
                    self.#field.clone()
                }
            })
        }

        // Create setter_opts
        let setter_opt_name = format_ident!("set_opts_{}", field.clone());
        all_setter_opts.push(quote::quote! {
            pub fn #setter_opt_name(mut self, value: &Option<#inner_ty>) -> Self {
                if let Some(value) = value.clone() {
                    self.#field = nulls::new(value);
                }

                self
            }
        });

        // Create setters
        let setter_name = format_ident!("set_{}", field.clone());
        let inner_ty_str = derive_utils::derive_type_to_string(&inner_ty);

        match inner_ty_str.as_str() {
            "String" => {
                all_setters.push(quote::quote! {
                    pub fn #setter_name<T: ToString>(mut self, value: T) -> Self {
                        self.#field = nulls::new(value.to_string());

                        self
                    }
                });
            },
            "Vec<String>" => {
                all_setters.push(quote::quote! {
                    pub fn #setter_name<T: ToString>(mut self, value: Vec<T>) -> Self {
                        let value: Vec<String> = value
                            .into_iter()
                            .map(|v| v.to_string())
                            .filter(|s| !s.is_empty())
                            .collect();

                        self.#field = nulls::new(value);

                        self
                    }
                });
            },
            _ => {
                all_setters.push(quote::quote! {
                    pub fn #setter_name(mut self, value: #inner_ty) -> Self {
                        self.#field = nulls::new(value);

                        self
                    }
                });
            }
        }


        if field.to_string().as_str() == "id" {
            let setter_name = format_ident!("set_insert_id");
            all_setters.push(quote::quote!{
                pub fn #setter_name<T>(mut self, size: T) -> Self
                where
                    T: ToString
                {
                    let size = size.to_string();
                    let id = self.id().unwrap_or_default();

                    if id.is_empty() {
                        let id = match size.to_lowercase().as_str() {
                            "sm" => ids::sm(),
                            "md" => ids::md(),
                            "lg" => ids::lg(),
                            _ => ids::max(),
                        };

                        self.id = nulls::new(id.to_string());
                    }

                    self
                }
            });
        }

        // All clones
        let clone_name = format_ident!("clone_{}", field.clone());
        match ty_to_str.to_lowercase().starts_with("string") {
            true => all_clones.push(quote::quote! {
                pub fn #clone_name(mut self, value: #ty) -> Self {
                    self.#field = value;

                    self
                }
            }),
            false => all_clones.push(quote::quote! {
                pub fn #clone_name(mut self, value: &#ty) -> Self {
                    self.#field = value.clone();

                    self
                }
            })
        }

        // All Null ⟶ Undefined
        let clear_name = format_ident!("clear_{}", field.clone());
        if ty_to_str.to_lowercase().starts_with("null<") {
            all_cleable_fields.push(field.clone());
            all_clears.push(quote::quote! {
                pub fn #clear_name(mut self) -> Self {
                    self.#field = nulls::undefined();

                    self
                }
            });
        }

        // Check if is_attributed
        if is_attributed {
            // Create basic table names and aliases
            let plain = derive_utils::derive_snake_case(field.clone().to_string());
            let renamed = format!("{}_{}", table_name, plain);
            let tabled = format!("{}.{}", table_name, plain);
            let aliased = format!("{} AS {}", tabled, renamed);

            all_attributed_fields.push(field.clone());
            all_attributed_inner_ty.push(inner_ty.clone());
            all_attributed_renamed.push(renamed.clone());

            all_const_names.push(format_ident!("{}", plain.to_uppercase()));
            all_aliased.push(aliased);
            all_plain.push(plain.clone());
            all_renamed.push(renamed.clone());
            all_tabled.push(tabled.clone());

            for a in aliases.clone() {
                let aliased_parser = format_ident!("parse_{}", a);
                let aliased_renamed = format!("{}_{}", a, plain);
                let sub_aliased = format!("{} AS {}", tabled, aliased_renamed);

                map_sub_parser.entry(aliased_parser.clone())
                    .and_modify(|d| d.push((field.clone(), inner_ty.clone(), aliased_renamed.clone())))
                    .or_insert(vec![(field.clone(), inner_ty.clone(), aliased_renamed.clone())]);

                map_sub_alias.entry(aliased_parser.clone())
                    .and_modify(|d| d.push(sub_aliased.clone()))
                    .or_insert(vec![sub_aliased]);
            }
        }
    }

    // Use explicit string join with &str type
    let all_aliased_str = all_aliased.join(", ");
    let all_plain_str = all_plain.join(", ");
    let all_renamed_str = all_renamed.join(", ");
    let all_tabled_str = all_tabled.join(", ");

    // Create Sub Alias
    //____________________________________________________________
    let mut sub_alias = Vec::<TS2>::new();  // Specify type explicitly
    for (k, v) in map_sub_alias {
        let all_alias_str = v.join(", ");
        let module = format_ident!("{}", k.to_string().replace("parse_", ""));

        sub_alias.push(quote::quote!{
            pub mod #module {
                pub const ALL: &'static str = #all_alias_str;

                #(
                    pub const #all_const_names: &'static str = #v;
                )*
            }
        });
    }

    // Create Sub Parsers
    //____________________________________________________________
    let mut sub_parsers = Vec::<TS2>::new();  // Specify type explicitly
    let mut sub_parser_mod = Vec::<TS2>::new();  // Specify type explicitly
    for (k, v) in map_sub_parser {
        let mut fields = Vec::<Ident>::new();  // Specify type explicitly
        let mut inner_ty = Vec::<Type>::new();  // Specify type explicitly
        let mut aliases = Vec::<String>::new();  // Specify type explicitly

        let module = format_ident!("{}", k.to_string().replace("parse_", ""));

        for (f, it, ar) in v {
            fields.push(f);
            inner_ty.push(it);
            aliases.push(ar);
        }

        sub_parsers.push(quote::quote! {
            pub fn #k(row: &sqlx::postgres::PgRow) -> Self {
                 use sqlx::Row;

                let mut data = Self::default();

                #(
                    data.#fields = nulls::Null::from(row.try_get::<#inner_ty, &str>(#aliases));
                )*

                data
            }
        });

        sub_parser_mod.push(quote::quote!{
            pub mod #module {
                use nulls::Null;
                use sqlx::{Result, Row, postgres::PgRow};

                use crate::schemas::#node;

                pub fn parse(row: &PgRow) -> #node {
                    #node::#k(row)
                }

                pub fn result(row: Result<sqlx::postgres::PgRow>) -> responder::Result<#node> {
                    let result = row.map_err(responder::query)?;
                    let row = parse(&result);

                    match !row.is_empty() {
                        true => Ok(row),
                        false => Err(responder::to(#error))
                    }
                }

                pub fn relational(row: &PgRow) -> Null<#node> {
                    let row = parse(row);

                    match row.is_empty() {
                        true => nulls::undefined(),
                        false => nulls::new(row)
                    }
                }
            }
        });
    }

    // Create Sub-module Implementations
    //____________________________________________________________
    token.extend(quote::quote!{
        pub mod alias {
            pub const ALL: &'static str = #all_aliased_str;

            #(
                pub const #all_const_names: &'static str = #all_aliased;
            )*


            #(#sub_alias)*
        }

        pub mod plain {
            pub const ALL: &'static str = #all_plain_str;

            #(
                pub const #all_const_names: &'static str = #all_plain;
            )*
        }

        pub mod renamed {
            pub const ALL: &'static str = #all_renamed_str;

            #(
                pub const #all_const_names: &'static str = #all_renamed;
            )*
        }

        pub mod tabled {
            pub const ALL: &'static str = #all_tabled_str;

            #(
                pub const #all_const_names: &'static str = #all_tabled;
            )*
        }

        pub mod parsers {
            use nulls::Null;
            use sqlx::{Result, Row, postgres::PgRow};

            use crate::schemas::#node;

            pub fn parse(row: &PgRow) -> #node {
                #node::parse(row)
            }

            pub fn result(row: Result<sqlx::postgres::PgRow>) -> responder::Result<#node> {
                let result = row.map_err(responder::query)?;
                let row = parse(&result);

                match !row.is_empty() {
                    true => Ok(row),
                    false => Err(responder::to(#error))
                }
            }

            pub fn relational(row: &PgRow) -> Null<#node> {
                let row = parse(row);

                match row.is_empty() {
                    true => nulls::undefined(),
                    false => nulls::new(row)
                }
            }

            #(#sub_parser_mod)*
        }
    });


    // Create Node Related implementations
    //____________________________________________________________
    token.extend(quote::quote!{
        impl #node {
            pub fn is_empty(&self) -> bool {
                *self == Self::default()
            }

            pub fn to<T>(&self) -> T
            where
                T: From<Self>
            {
                T::from(self.clone())
            }

            pub fn to_json(&self) -> serde_json::Value {
                serde_json::to_value(self)
                    .unwrap_or(serde_json::Value::Null)
            }

            pub fn to_jsonb(&self) -> sqlx::types::Json<Self> {
                sqlx::types::Json::from(self.clone())
            }

            #(#all_props)*

            #(#all_setters)*

            #(#all_setter_opts)*

            #(#all_clones)*

            #(#all_clears)*

            pub fn clear_all(mut self) -> Self {
                #(
                    if !self.#all_cleable_fields.is_some() {
                        self.#all_cleable_fields =  nulls::undefined();
                    }
                )*

                self
            }

            pub fn parse(row: &sqlx::postgres::PgRow) -> Self {
                use sqlx::Row;

                let mut data = Self::default();

                #(
                    data.#all_attributed_fields = nulls::Null::from(row.try_get::<#all_attributed_inner_ty, &str>(#all_attributed_renamed));
                )*


                data
            }

            #(#sub_parsers)*

            pub async fn update(&self) -> responder::Result<Self> {
                let mut index = 0;
                let mut updates = Vec::<String>::new();  // Specify type explicitly

                 #(
                    if self.#all_update_fields.is_some() || self.#all_update_fields.is_none() {
                        index += 1;
                        updates.push(format!(#all_update_columns, index));
                    }
                )*

                index += 1;
                let sql = format!(r#"
                    UPDATE {} SET {} WHERE id = ${} RETURNING {}
                "#, #table_name, updates.join(", "), index, alias::ALL);

                let mut query = sqlx::query(&sql);

                #(
                    if self.#all_update_fields.is_some() || self.#all_update_fields.is_none() {
                        query = query.bind(self.#all_update_fields());
                    }
                )*

                query = query.bind(self.id());
                parsers::result(query.fetch_one(services::database::writer()).await)
            }
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
    });


    // Return the new token
    Ok(token)
}