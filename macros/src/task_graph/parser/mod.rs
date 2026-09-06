use proc_macro2::TokenStream as TokenStream2;

use super::ast::TaskGraphAst;

#[cfg(test)]
mod tests;

pub(super) fn parse(input: TokenStream2) -> Result<TaskGraphAst, &'static str> {
    Err("placeholder")
}
