use proc_macro::TokenStream;
use quote::quote;

use super::ast::TaskGraphAst;

pub(super) fn compile(ast: TaskGraphAst) -> TokenStream {
    let out_stream = quote! {};

    out_stream.into()
}
