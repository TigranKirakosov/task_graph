use core;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use super::ast::{self, NodeExpr, Task};

#[derive(Default)]
struct Context {
    decls: Vec<TokenStream2>,
    links: Vec<TokenStream2>,
    anon_counter: usize,
}

pub(super) fn generate(ast: ast::TaskGraphAst) -> TokenStream {
    let mut cx = Context::default();

    for g in ast.graphs {
        cx.process_graph(g);
    }

    let Context { decls, links, .. } = cx;
    let out_stream = quote! {
        {
             #(#decls)*
             #(#links)*
        }
    };

    out_stream.into()
}

impl Context {
    fn process_graph(&mut self, g: ast::Graph) {
        self.process_node(g.entry);

        for conn in g.conns {
            self.process_node(conn);
        }
    }

    fn process_node(&mut self, n: NodeExpr) {
        match n {
            NodeExpr::Declaration(task) => self.declare(task),
            NodeExpr::Binding(var) => {}
            NodeExpr::Group(group) => {
                for g in group.graphs {
                    self.process_graph(g);
                }
            }
        }
    }

    fn declare(&mut self, Task { var, typ }: Task) {
        let decl = if let Some(var) = var {
            quote! {
                let #var = graph.add_node::<#typ>();
            }
        } else {
            let anon = format_ident!("anon_{}", self.anon_counter);
            self.anon_counter += 1;

            quote! {
                let #anon = graph.add_node::<#typ>();
            }
        };

        self.decls.push(decl);
    }
}
