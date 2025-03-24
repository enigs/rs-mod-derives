use proc_macro::TokenStream;
use syn::DeriveInput;

// Entry point for our macro
#[proc_macro_derive(IsEmpty)]
pub fn main(stream: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(stream).unwrap();
    let node = ast.ident;

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
    })
}