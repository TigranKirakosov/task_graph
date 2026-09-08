use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, format_ident, quote};
use std::collections::{HashMap, HashSet};
use syn::{Ident, spanned::Spanned};

use super::ast::{self, GroupBlock, NodeExpr, SchedulingMode, Task};

struct DummyMarker;

#[derive(Default)]
struct Context {
    compile_graph: core::Graph,
    decls: Vec<TokenStream2>,
    links: Vec<TokenStream2>,
    id_map: HashMap<Ident, usize>,
    unbound_types: HashSet<String>,
    anon_counter: usize,
}

pub(super) enum CodegenError {
    DuplicateUnboundType {
        type_key: String,
        span: proc_macro2::Span,
    },
    CircularDependency {
        from: Ident,
        to: Ident,
        span: proc_macro2::Span,
    },
    Syn(syn::Error),
}

type Roots = Vec<Ident>;
type Leaves = Vec<Ident>;

pub(super) fn generate(ast: ast::TaskGraphAst) -> TokenStream {
    let mut cx = Context::default();

    for graph in ast.graphs {
        if let Err(codegen_err) = cx.process_graph(graph) {
            let syn_err = syn::Error::from(codegen_err);
            let err_msg = syn_err.to_string();
            let err_span = syn_err.span();

            let out_stream = quote::quote_spanned! { err_span =>
                ::std::compile_error!(#err_msg)
            };

            return out_stream.into();
        }
    }

    let Context { decls, links, .. } = cx;
    let out_stream = quote! {
        {
            let mut graph = Graph::new();
            #(#decls)*
            #(#links)*
            graph
        }
    };

    out_stream.into()
}

impl Context {
    fn process_graph(&mut self, graph: ast::Graph) -> Result<(Roots, Leaves), CodegenError> {
        let (roots, mut prev_leaves) = self.process_node(graph.entry)?;

        for conn in graph.conns {
            let node_span = match &conn {
                NodeExpr::Declaration(task) => task
                    .var
                    .as_ref()
                    .map(|v| v.span())
                    .unwrap_or_else(|| proc_macro2::Span::call_site()),
                NodeExpr::Binding(var) => var.span(),
                NodeExpr::Group(block) => block.span_info.span,
            };
            let (local_roots, local_leaves) = self.process_node(conn)?;

            for from in &prev_leaves {
                for to in &local_roots {
                    self.try_add_edge(node_span, from, to)?;

                    self.links.push(quote! {
                        graph.add_edge(#from, #to);
                    });
                }
            }

            prev_leaves = local_leaves
        }

        Ok((roots, prev_leaves))
    }

    fn process_node(&mut self, node: NodeExpr) -> Result<(Roots, Leaves), CodegenError> {
        match node {
            NodeExpr::Declaration(task) => {
                let ident = vec![self.declare(task)?];
                Ok((ident.clone(), ident))
            }
            NodeExpr::Binding(var) => Ok((vec![var.clone()], vec![var])),
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
    ) -> Result<(Roots, Leaves), CodegenError> {
        match mode {
            SchedulingMode::Parallel => {
                let mut acc_roots = Vec::new();
                let mut acc_leaves = Vec::new();

                for g in graphs {
                    let (roots, leaves) = self.process_graph(g)?;
                    acc_roots.extend(roots);
                    acc_leaves.extend(leaves);
                }

                Ok((acc_roots, acc_leaves))
            }
            SchedulingMode::Sequence => {
                let mut graphs = graphs.into_iter();

                let (roots, mut prev_leaves) = self.process_graph(graphs.next().unwrap())?;
                for graph in graphs {
                    let (local_roots, local_leaves) = self.process_graph(graph)?;

                    for from in &prev_leaves {
                        for to in &local_roots {
                            self.try_add_edge(span_info.span, from, to)?;

                            self.links.push(quote! {
                                graph.add_edge(#from, #to);
                            });
                        }
                    }

                    prev_leaves = local_leaves
                }

                Ok((roots, prev_leaves))
            }
        }
    }

    fn declare(&mut self, Task { var, typ }: Task) -> Result<Ident, CodegenError> {
        let type_key = typ.to_token_stream().to_string().replace(" ", "");

        if var.is_none() {
            if self.unbound_types.contains(&type_key) {
                return Err(CodegenError::DuplicateUnboundType {
                    type_key,
                    span: typ.span(),
                });
            }
            self.unbound_types.insert(type_key.clone());
        }

        let var_iden = var.unwrap_or_else(|| {
            let anon = format_ident!("anon_{}", self.anon_counter);
            self.anon_counter += 1;

            anon
        });

        let decl = quote! {
            let #var_iden = graph.add_node::<#typ>();
        };
        self.decls.push(decl);

        let node_id = self.compile_graph.add_node::<DummyMarker>();
        self.id_map.insert(var_iden.clone(), node_id);

        Ok(var_iden)
    }

    fn try_add_edge(&mut self, span: Span, from: &Ident, to: &Ident) -> Result<(), CodegenError> {
        let from_id = *self.id_map.get(from).expect("Missing source node lookup");
        let to_id = *self.id_map.get(to).expect("Missing target node lookup");

        self.compile_graph.add_edge(from_id, to_id);

        if let Err(core::GraphError::CycleDetected) = self.compile_graph.sort_ordered() {
            return Err(CodegenError::CircularDependency {
                from: from.clone(),
                to: to.clone(),
                span,
            });
        }
        Ok(())
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
