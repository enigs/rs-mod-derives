use change_case::snake_case;
use deluxe::{extract_attributes, ExtractAttributes};
use proc_macro2::Ident;
use regex::Regex;
use syn::{parse_str, Data, DeriveInput, Field, Fields, Type};

pub trait Pagination<T> {
    fn page(&self) -> i64;
    fn per_page(&self) -> i64;
    fn filtered_count(&self) -> i64;
    fn total_count(&self) -> i64;
    fn records(&self) -> Vec<T>;
}

/// Derives all fields of a struct along with their attributes.
///
/// This function extracts all fields from the struct, checking if each field has
/// the specified attribute and retrieving the associated column attributes. It
/// returns a list of tuples, each containing the field's identifier, type, a
/// boolean indicating if the field has the attribute, and the column attributes.
///
/// # Type Parameters
/// - `T`: A type that implements `ToString`, representing the name of the attribute to search for.
///
/// # Arguments
/// - `ast`: A reference to the `DeriveInput`, which represents the abstract syntax tree (AST) of the struct.
/// - `name`: The name of the attribute to check for, as a type `T`.
///
/// # Returns
/// - A `Vec` of tuples, where each tuple contains:
///     - The field's identifier (`Ident`),
///     - The field's type (`Type`),
///     - A boolean indicating whether the field has the specified attribute,
///     - The column attributes (`ColumnAttrs`).
///
/// # Example
/// ```rust
/// use syn::{parse_quote, DeriveInput};
/// use quote::format_ident;
///
/// let ast: DeriveInput = parse_quote! {
///     struct MyStruct {
///         #[my_attribute]
///         field1: String,
///         field2: i32,
///     }
/// };
/// let result = derive_utils::derive_all_fields(&ast, "my_attribute");
/// assert_eq!(result.len(), 2);
/// ```
pub fn derive_all_fields<T, U>(ast: &DeriveInput, name: T) -> Vec<(Ident, Type, bool, U)>
where
    T: ToString,
    U: ExtractAttributes<Field> + Default
{
    let name = name.to_string();
    let mut result = Vec::new();

    if let Data::Struct(s) = &mut ast.data.clone() {
        for field in s.fields.iter_mut() {
            if let Some(ident) = &field.clone().ident {
                if let Ok(attrs) = extract_attributes(&mut field.clone()) {
                    let has_attribute = derive_is_attributed_field(
                        ast,
                        ident.to_string(),
                        &name
                    );

                    result.push((
                        ident.clone(),
                        field.ty.clone(),
                        has_attribute,
                        attrs,
                    ));
                } else {
                    result.push((
                        ident.clone(),
                        field.ty.clone(),
                        false,
                        U::default(),
                    ));
                }
            }
        }
    }

    result
}

/// Parses the inner type from a type string, if applicable.
///
/// This function takes a reference to a `Type`, converts it to a string representation,
/// and attempts to parse the inner type if the type is a generic. If the type is not a generic,
/// it attempts to parse the type directly from the string. If parsing fails, it will panic.
///
/// The function uses regular expressions to identify if the type is a generic (e.g., `Option<T>`)
/// and extracts the inner type (e.g., `T`).
///
/// # Arguments
/// - `ty`: A reference to the type that is to be parsed.
///
/// # Returns
/// - The inner type if it is a generic type, or the type itself if it's not a generic.
///
/// # Panics
/// - Panics if the type string is invalid and cannot be parsed.
///
/// # Example
/// ```rust
/// let ty: syn::Type = syn::parse_str("Option<i32>").unwrap();
/// let inner_type = derive_utils::derive_parse_inner_type(&ty);
/// // inner_type is now `i32`
/// ```
pub fn derive_parse_inner_type(ty: &Type) -> Type {
    let input = derive_type_to_string(ty);

    let re = Regex::new(r"^[^<]*<(.+)>$").unwrap();
    if let Some(captures) = re.captures(&input) {
        if let Some(captured) = captures.get(1) {
            if let Ok(ty) = parse_str::<Type>(captured.as_str()) {
                return ty;
            }
        }
    } else if let Ok(ty) = parse_str::<Type>(&input) {
        return ty;
    }

    panic!("Invalid type string");
}

/// Checks if a attributed field in a struct has a specific attribute.
///
/// This function checks if a field within a struct, which has attributed fields, contains
/// a specific attribute. It looks for both the field name and the attribute name.
///
/// # Type Parameters
/// - `T`: A type that implements `ToString` representing the field name (e.g., `"field_name"`).
/// - `U`: A type that implements `ToString` representing the attribute name (e.g., `"my_attribute"`).
///
/// # Arguments
/// - `ast`: A reference to the `DeriveInput`, which represents the abstract syntax tree (AST) of the struct.
/// - `field`: The name of the field to check for, as a type `T` (e.g., `"field_name"`).
/// - `name`: The name of the attribute to check for, as a type `U` (e.g., `"my_attribute"`).
///
/// # Returns
/// - `true` if the struct contains the specified attributed field with the specified attribute.
/// - `false` otherwise.
///
/// # Example
/// ```rust
/// use syn::{parse_quote, DeriveInput};
///
/// let ast: DeriveInput = parse_quote! {
///     struct MyStruct {
///         #[my_attribute]
///         field_name: String,
///     }
/// };
///
/// assert!(derive_utils::derive_is_attributed_field(&ast, "field_name", "my_attribute"));
/// assert!(!derive_utils::derive_is_attributed_field(&ast, "other_field", "my_attribute"));
/// assert!(!derive_utils::derive_is_attributed_field(&ast, "field_name", "other_attribute"));
/// ```
pub fn derive_is_attributed_field<T, U>(ast: &DeriveInput, field: T, name: U) -> bool
where
    T: ToString,
    U: ToString,
{
    let field = field.to_string();
    let name = name.to_string();

    if let Data::Struct(data_struct) = &ast.data {
        if let Fields::Named(fields) = &data_struct.fields {
            return fields.named.iter().any(|f| {
                f.ident
                    .as_ref()
                    .map(|ident| ident == &field)
                    .unwrap_or(false)
                    && f.attrs.iter().any(|attr| attr.path().is_ident(&name))
            });
        }
    }

    false
}

/// Determines if a struct has a specific attribute.
///
/// This function checks if the given struct's attributes contain the specified attribute name.
/// It is commonly used in procedural macros to identify attributes on a struct.
///
/// # Type Parameters
/// - `T`: A type that implements `ToString`, representing the attribute name.
///
/// # Arguments
/// - `ast`: A reference to the `DeriveInput`, representing the abstract syntax tree of the struct.
/// - `name`: The name of the attribute to look for.
///
/// # Returns
/// - `true` if the struct has an attribute with the specified name.
/// - `false` otherwise.
///
/// # Example
/// ```rust
/// use syn::{parse_quote, DeriveInput};
///
/// let ast: DeriveInput = parse_quote! {
///     #[my_attribute]
///     struct MyStruct;
/// };
///
/// assert!(derive_utils::derive_is_attributed_struct(&ast, "my_attribute"));
/// assert!(!derive_utils::derive_is_attributed_struct(&ast, "non_existent"));
/// ```
pub fn derive_is_attributed_struct<T>(ast: &DeriveInput, name: T) -> bool
where
    T: ToString
{
    let name = name.to_string();
    let attr = ast.attrs.clone();

    for a in &attr {
        if a.path().is_ident(&name) {
            return true;
        }
    }

    false
}

/// Converts a given name to snake_case (lowercase, with underscores).
///
/// This function takes a string or any type that can be converted to a string
/// and converts it to snake_case, which is a common convention in Rust and
/// other programming languages for variable and function names. The resulting
/// string is also converted to lowercase.
///
/// # Type Parameters
/// - `T`: A type that implements `ToString`, representing the input name to convert.
///
/// # Arguments
/// - `name`: The input name (e.g., "MyVariable", "MyFunction") to convert to snake_case.
///
/// # Returns
/// - A `String` in snake_case format, converted to lowercase (e.g., "my_variable", "my_function").
///
/// # Example
/// ```rust
/// assert_eq!(derive_utils::derive_snake_case("MyVariable"), "my_variable");
/// assert_eq!(derive_utils::derive_snake_case("CamelCaseExample"), "camel_case_example");
/// assert_eq!(derive_utils::derive_snake_case("Some Function Name"), "some_function_name");
/// ```
///
/// # Notes
/// - This function assumes that the input string uses a typical naming convention such as camelCase or PascalCase.
///
/// # Dependencies
/// - This function relies on an external crate or a custom implementation of `snake_case()`.
pub fn derive_snake_case<T>(name: T) -> String
where
    T: ToString
{
    snake_case(&name.to_string()).to_lowercase()
}

/// Extracts attributes from a struct's derive input.
///
/// This function attempts to extract attributes from the given `DeriveInput`
/// using a generic type `T` that implements the `ExtractAttributes` trait.
/// If the extraction fails (e.g., due to an error during parsing), it returns
/// the default value of `T`.
///
/// # Type Parameters
/// - `T`: A type that implements `Default` and `ExtractAttributes<DeriveInput>`.
///
/// # Arguments
/// - `ast`: A reference to the `DeriveInput`, representing the abstract syntax tree of the struct.
///
/// # Returns
/// - The extracted attributes of type `T` if the extraction is successful.
/// - The default value of `T` if the extraction fails.
///
/// # Example
/// ```rust
/// use deluxe::ExtractAttributes;
/// use syn::{parse_quote, DeriveInput, LitStr};
///
/// #[derive(Default, Debug, ExtractAttributes)]
/// #[deluxe(attributes(foo))]
/// struct FooAttrs {
///     bar: Option<LitStr>
/// }
///
/// // Define a struct and its attributes.
/// let ast: DeriveInput = parse_quote! {
///      #[derive(Default, Debug)]
///      #[foo(rename="Lorem")]
///      struct Foo {
///         baz: Option<String>
///      }
/// };
///
/// // Assuming `MyAttributes` implements `Default` and `ExtractAttributes<DeriveInput>`.
/// let attributes: FooAttrs = derive_utils::derive_struct_attrs(&ast);
/// ```
pub fn derive_struct_attrs<T>(ast: &DeriveInput) -> T
where
    T: Default + ExtractAttributes<DeriveInput>
{
    let result: Result<T, syn::Error> = extract_attributes(&mut ast.clone());
    if let Ok(table_attrs) = result {
        return table_attrs;
    }

    T::default()
}

/// Converts the given `Type` to a string representation.
///
/// This function takes a reference to a `Type` and generates a string that represents
/// the type using the `quote!` macro. It then removes any spaces from the generated string
/// to return a compact representation of the type.
///
/// # Arguments
/// - `ty`: A reference to a `Type` that you want to convert to a string.
///
/// # Returns
/// - A `String` representing the type, with spaces removed.
///
/// # Example
/// ```rust
/// let ty = syn::parse_str::<syn::Type>("i32").unwrap();
/// let ty_str = derive_utils::derive_type_to_string(&ty);
/// println!("{}", ty_str); // Output: "i32"
/// ```
pub fn derive_type_to_string(ty: &Type) -> String {
    format!("{}", quote::quote! { #ty }).replace(" ", "")
}



