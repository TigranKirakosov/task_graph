use proc_macro::TokenStream;
use quote::quote;

mod ast;
mod compiler;
mod parser;

pub fn task_graph(input: TokenStream) -> TokenStream {
    match parser::parse(input.into()) {
        Ok(ast) => compiler::compile(ast),
        Err(err) => {
            let error_msg = format!("Task graph parsing error: {:?}", err);
            quote! { compile_error!(#error_msg); }.into()
        }
    }
}
