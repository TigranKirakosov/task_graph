use proc_macro::TokenStream;

mod task_graph;

#[proc_macro]
pub fn task_graph(input: TokenStream) -> TokenStream {
    task_graph::task_graph(input)
}
