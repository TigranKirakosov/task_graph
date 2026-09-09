use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use syn::{Ident, spanned::Spanned};

use crate::task_graph::format_type;

use super::ast::{self, GroupBlock, NodeExpr, SchedulingMode, Task};

type TypeStr = String;

struct DummyMarker;

#[derive(Default)]
struct Context {
    compile_graph: core::Graph,
    decls: Vec<TokenStream2>,
    links: Vec<TokenStream2>,
    id_map: HashMap<Ident, usize>,
    anon_map: HashMap<Ident, TypeStr>,
    unbound_types: HashSet<TypeStr>,
    anon_counter: usize,
    errors: Vec<CodegenError>,
}

pub(super) enum CodegenError {
    DuplicateUnboundType {
        type_key: String,
        span: Span,
    },
    CircularDependency {
        from: String,
        to: String,
        span: Span,
    },
    Syn(syn::Error),
}

type Roots = Vec<Ident>;
type Leaves = Vec<Ident>;

pub(super) fn generate(ast: ast::TaskGraphAst) -> TokenStream {
    let mut cx = Context::default();

    for graph in ast.graphs {
        let _ = cx.process_graph(graph);
    }

    let Context {
        decls,
        links,
        errors,
        ..
    } = cx;

    let compile_errors = errors.into_iter().map(|codegen_err| {
        let syn_err = syn::Error::from(codegen_err);
        let err_msg = syn_err.to_string();
        let err_span = syn_err.span();

        quote::quote_spanned! { err_span =>
            compile_error!{#err_msg};
        }
    });

    let out_stream = quote! {
       {
           let mut graph = Graph::new();
            #(#decls)*
            #(#links)*
            #(#compile_errors)*
            graph
       }
    };

    out_stream.into()
}

impl Context {
    fn process_graph(&mut self, graph: ast::Graph) -> (Roots, Leaves) {
        let (roots, mut prev_leaves) = self.process_node(graph.entry);

        for conn in graph.conns {
            let node_span = match &conn {
                NodeExpr::Declaration(task) => task
                    .var
                    .as_ref()
                    .map(|v| v.span())
                    .unwrap_or_else(|| task.typ.span()),
                NodeExpr::Binding(var) => var.span(),
                NodeExpr::Group(block) => block.span_info.span,
            };

            let (local_roots, local_leaves) = self.process_node(conn);
            for from in &prev_leaves {
                for to in &local_roots {
                    self.track_edge(node_span, from, to);

                    self.links.push(quote! {
                        graph.add_edge(#from, #to);
                    });
                }
            }

            prev_leaves = local_leaves
        }

        (roots, prev_leaves)
    }

    fn process_node(&mut self, node: NodeExpr) -> (Roots, Leaves) {
        match node {
            NodeExpr::Declaration(task) => {
                let ident = vec![self.declare(task)];
                (ident.clone(), ident)
            }
            NodeExpr::Binding(var) => (vec![var.clone()], vec![var]),
            NodeExpr::Group(group) => self.process_group(group),
        }
    }

    fn process_group(
        &mut self,
        GroupBlock {
            mode,
            graphs,
            span_info,
        }: GroupBlock,
    ) -> (Roots, Leaves) {
        match mode {
            SchedulingMode::Parallel => {
                let mut acc_roots = Vec::new();
                let mut acc_leaves = Vec::new();

                for g in graphs {
                    let (roots, leaves) = self.process_graph(g);
                    acc_roots.extend(roots);
                    acc_leaves.extend(leaves);
                }

                (acc_roots, acc_leaves)
            }
            SchedulingMode::Sequence => {
                let mut graphs = graphs.into_iter();

                let (roots, mut prev_leaves) = self.process_graph(graphs.next().unwrap());
                for graph in graphs {
                    let (local_roots, local_leaves) = self.process_graph(graph);

                    for from in &prev_leaves {
                        for to in &local_roots {
                            self.track_edge(span_info.span, from, to);

                            self.links.push(quote! {
                                graph.add_edge(#from, #to);
                            });
                        }
                    }

                    prev_leaves = local_leaves
                }

                (roots, prev_leaves)
            }
        }
    }

    fn declare(&mut self, Task { var, typ }: Task) -> Ident {
        let type_key = format_type(&typ);

        if var.is_none() {
            if self.unbound_types.contains(&type_key) {
                self.errors.push(CodegenError::DuplicateUnboundType {
                    type_key: type_key.clone(),
                    span: typ.span(),
                });
            }
            self.unbound_types.insert(type_key.clone());
        }

        let var_iden = var.unwrap_or_else(|| {
            let anon = format_ident!("anon_{}", self.anon_counter);
            self.anon_map.insert(anon.clone(), type_key);
            self.anon_counter += 1;

            anon
        });

        let decl = quote! {
            let #var_iden = graph.add_node::<#typ>();
        };
        self.decls.push(decl);

        let node_id = self.compile_graph.add_node::<DummyMarker>();
        self.id_map.insert(var_iden.clone(), node_id);

        var_iden
    }

    fn track_edge(&mut self, span: Span, from: &Ident, to: &Ident) {
        let from_id = *self.id_map.get(from).expect("Missing source node lookup");
        let to_id = *self.id_map.get(to).expect("Missing target node lookup");

        self.compile_graph.add_edge(from_id, to_id);

        if let Err(core::GraphError::CycleDetected) = self.compile_graph.sort_ordered() {
            let from = self.anon_map.get(from).unwrap_or(&from.to_string()).clone();

            self.errors.push(CodegenError::CircularDependency {
                from,
                to: to.to_string(),
                span,
            });
        }
    }
}

impl From<syn::Error> for CodegenError {
    fn from(err: syn::Error) -> Self {
        CodegenError::Syn(err)
    }
}

impl From<CodegenError> for syn::Error {
    fn from(err: CodegenError) -> syn::Error {
        match err {
            CodegenError::DuplicateUnboundType { type_key, span } => syn::Error::new(
                span,
                format!(
                    "Task Graph Error: Duplicate unbound type declaration: {type_key}.\n\
                    Multiple instances of the same task type must be assigned to unique variables (e.g., foo: {type_key} -> bar: {type_key})."
                ),
            ),
            CodegenError::CircularDependency { from, to, span } => syn::Error::new(
                span,
                format!("Task Graph Error: Circular dependency: {from} -> {to}"),
            ),
            CodegenError::Syn(syn_err) => syn_err,
        }
    }
}
