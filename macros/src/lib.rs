use proc_macro::TokenStream;

mod orchestrator;

#[proc_macro]
pub fn orc(input: TokenStream) -> TokenStream {
    orchestrator::orc(input)
}
