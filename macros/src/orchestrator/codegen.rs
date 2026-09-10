use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use std::collections::{HashMap, HashSet};
use syn::{Ident, spanned::Spanned};

use super::{
    ast::{self, GroupBlock, NodeExpr, SchedulingMode, Task},
    format_type,
};

#[derive(Default)]
struct Context {
    graph_ident: GraphIdent,
    compile_graph: action_orc_core::Graph,
    decls: Vec<TokenStream2>,
    links: Vec<TokenStream2>,
    parallel_group_id_counter: usize,
    anon_id_counter: usize,
    node_id_map: HashMap<Ident, usize>,
    anon_map: HashMap<Ident, TypeStr>,
    unbound_types: HashSet<TypeStr>,
    embeddings: HashSet<Ident>,
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
    VariableCollision {
        var: String,
        span: Span,
    },
    Syn(syn::Error),
}

enum NodeBound {
    /// Source or sink of currently building graph node
    Literal(Vec<Ident>),
    /// Embedded #[graph]
    Graph(Ident),
    /// Synthetic runtime vec reference, built like `let mut #group_0_sink = vec![...];`
    ParallelGroup(Ident),
}

struct IdFactory;
struct DummyMarker;
struct GraphIdent(Ident);
struct Source(NodeBound);
struct Sink(NodeBound);

type TypeStr = String;

pub(super) fn generate(ast: ast::SyntaxTree) -> TokenStream {
    let mut cx = Context::default();

    for graph in ast.graphs {
        let _ = cx.process_graph(graph);
    }

    let Context {
        graph_ident: GraphIdent(graph),
        decls,
        links,
        errors,
        ..
    } = cx;

    let compile_errors = errors.into_iter().map(|codegen_err| {
        let syn_err = syn::Error::from(codegen_err);
        let err_msg = syn_err.to_string();
        let err_span = syn_err.span();

        quote_spanned! { err_span =>
            compile_error!{#err_msg};
        }
    });

    let out_stream = quote! {
       {
            let mut #graph = Graph::new();
            #(#decls)*
            #(#links)*
            #(#compile_errors)*
            #graph
       }
    };

    out_stream.into()
}

impl Context {
    fn process_graph(&mut self, graph: ast::Graph) -> (Source, Sink) {
        let (source, mut prev_sink) = self.process_node(graph.entry);

        // Edge case: graph starts with embedded sub-graph:
        // #[sub] -> (...)
        // Immediately merge it with master graph, storing its shifted endpoints
        if let NodeBound::Graph(sub) = &source.0 {
            let GraphIdent(graph) = &self.graph_ident;
            let (sub_source, sub_sink) = IdFactory::graph_bounds(sub);
            self.links.push(quote! {
                let merged = #graph.merge(&#sub, vec![]);
                let #sub_source = merged.sources;
                let #sub_sink = merged.sinks;
            });
        }

        for conn in graph.conns {
            let node_span = match &conn {
                NodeExpr::Declaration(task) => task
                    .var
                    .as_ref()
                    .map(|v| v.span())
                    .unwrap_or_else(|| task.typ.span()),
                NodeExpr::Binding(var) => var.span(),
                NodeExpr::Embedding(emb) => emb.span(),
                NodeExpr::Group(block) => block.span_info.span,
            };

            let (sub_source, sub_sink) = self.process_node(conn);
            self.track_edge(node_span, &prev_sink, &sub_source);
            self.stich_nodes(&prev_sink, &sub_source);

            prev_sink = sub_sink
        }

        (source, prev_sink)
    }

    fn process_node(&mut self, node: NodeExpr) -> (Source, Sink) {
        match node {
            NodeExpr::Declaration(task) => NodeBound::literal(self.declare(task)),
            NodeExpr::Binding(var) => NodeBound::literal(var),
            NodeExpr::Embedding(emb) => self.process_embedding(emb),
            NodeExpr::Group(group) => self.process_group(group),
        }
    }

    fn process_group(&mut self, GroupBlock { mode, graphs, .. }: GroupBlock) -> (Source, Sink) {
        match mode {
            SchedulingMode::Parallel => {
                let group_id = self.parallel_group_id_counter;
                self.parallel_group_id_counter += 1;

                let (group_source, group_sink) = IdFactory::parallel_group_bounds(group_id);

                self.links.push(quote! {
                    let mut #group_source = Vec::new();
                    let mut #group_sink = Vec::new();
                });

                for graph in graphs {
                    let (Source(source), Sink(sink)) = self.process_graph(graph);
                    match source {
                        NodeBound::Literal(ids) => {
                            self.links.push(quote! {
                                #group_source.extend(vec![#(#ids),*]);
                            });
                        }
                        NodeBound::Graph(sub) => {
                            let (sub_source, _) = IdFactory::graph_bounds(&sub);
                            self.links.push(quote! {
                                #group_source.extend(#sub_source);
                            });
                        }
                        NodeBound::ParallelGroup(group) => {
                            self.links.push(quote! {
                                #group_source.extend(#group);
                            });
                        }
                    }

                    match sink {
                        NodeBound::Literal(ids) => {
                            self.links.push(quote! {
                                #group_sink.extend(vec![#(#ids),*]);
                            });
                        }
                        NodeBound::Graph(sub) => {
                            let (_, sub_sink) = IdFactory::graph_bounds(&sub);
                            self.links.push(quote! {
                                #group_sink.extend(#sub_sink);
                            });
                        }
                        NodeBound::ParallelGroup(group) => {
                            self.links.push(quote! {
                                #group_sink.extend(#group);
                            });
                        }
                    }
                }

                (
                    Source(NodeBound::ParallelGroup(group_source)),
                    Sink(NodeBound::ParallelGroup(group_sink)),
                )
            }
            SchedulingMode::Sequence => {
                let mut graphs = graphs.into_iter();

                let (source, mut prev_sink) = self.process_graph(graphs.next().unwrap());
                for graph in graphs {
                    let span = graph.span_info.span;
                    let (sub_source, sub_sink) = self.process_graph(graph);
                    self.track_edge(span, &prev_sink, &sub_source);
                    self.stich_nodes(&prev_sink, &sub_source);

                    prev_sink = sub_sink
                }

                (source, prev_sink)
            }
        }
    }

    fn stich_nodes(&mut self, Sink(upstream): &Sink, Source(downstream): &Source) {
        let GraphIdent(graph) = &self.graph_ident;

        match (upstream, downstream) {
            // (A | B) -> (C | D)
            (NodeBound::Literal(from), NodeBound::Literal(to)) => {
                for f in from {
                    for t in to {
                        self.links.push(quote! {
                            #graph.add_edge(#f, #t);
                        });
                    }
                }
            }

            // #[G] -> #[H]
            (NodeBound::Graph(from), NodeBound::Graph(to)) => {
                let (_, f_sink) = IdFactory::graph_bounds(from);
                let (t_source, t_sink) = IdFactory::graph_bounds(to);
                self.links.push(quote! {
                    let merged = #graph.merge(&#to, #f_sink);
                    let #t_source = merged.sources;
                    let #t_sink = merged.sinks;
                });
            }

            // (A | B) -> #[H]
            (NodeBound::Literal(from), NodeBound::Graph(to)) => {
                let (t_source, t_sink) = IdFactory::graph_bounds(to);
                self.links.push(quote! {
                    let merged = #graph.merge(&#to, vec![#(#from),*]);
                    let #t_source = merged.sources;
                    let #t_sink = merged.sinks;
                });
            }

            // #[G] -> (C | D)
            (NodeBound::Graph(from), NodeBound::Literal(to)) => {
                let (_, f_sink) = IdFactory::graph_bounds(from);
                for t in to {
                    self.links.push(quote! {
                        for &f in &#f_sink {
                            #graph.add_edge(f, #t);
                        }
                    });
                }
            }

            // ((..) | (..)) -> C
            (NodeBound::ParallelGroup(from), NodeBound::Literal(to)) => {
                for to in to {
                    self.links.push(quote! {
                        for &from in &#from {
                            #graph.add_edge(from, #to);
                        }
                    });
                }
            }

            // C -> ((..) | (..))
            (NodeBound::Literal(from), NodeBound::ParallelGroup(to)) => {
                for f in from {
                    self.links.push(quote! {
                        for &t in &#to {
                            #graph.add_edge(#f, t);
                        }
                    });
                }
            }

            // ((..) | (..)) -> #[H]
            (NodeBound::ParallelGroup(from), NodeBound::Graph(to)) => {
                let (t_source, t_sink) = IdFactory::graph_bounds(to);
                self.links.push(quote! {
                    let merged = #graph.merge(&#to, #from.clone());
                    let #t_source = merged.sources;
                    let #t_sink = merged.sinks;
                });
            }

            // #[G] -> ((..) | (..))
            (NodeBound::Graph(from), NodeBound::ParallelGroup(to)) => {
                let (_, f_sink) = IdFactory::graph_bounds(from);
                self.links.push(quote! {
                    for &f in &#f_sink {
                        for &t in &#to {
                            #graph.add_edge(f, t);
                        }
                    }
                });
            }

            // ((..) | (..)) -> ((..) | (..))
            (NodeBound::ParallelGroup(from), NodeBound::ParallelGroup(to)) => {
                self.links.push(quote! {
                    for &f in &#from {
                        for &t in &#to {
                            #graph.add_edge(f, t);
                        }
                    }
                });
            }
        }
    }

    fn declare(&mut self, Task { var, typ }: Task) -> Ident {
        let GraphIdent(graph) = &self.graph_ident;
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

        let node_ident = var.unwrap_or_else(|| {
            let anon = IdFactory::anonymous_node(self.anon_id_counter);
            self.anon_map.insert(anon.clone(), type_key);
            self.anon_id_counter += 1;

            anon
        });

        let decl = quote! {
            let #node_ident = #graph.add_node::<#typ>();
        };
        self.decls.push(decl);

        let node_id = self.compile_graph.add_node::<DummyMarker>();
        if self.node_id_map.contains_key(&node_ident) {
            self.errors.push(CodegenError::VariableCollision {
                var: node_ident.to_string(),
                span: node_ident.span(),
            });
        }
        self.node_id_map.insert(node_ident.clone(), node_id);

        node_ident
    }

    fn process_embedding(&mut self, embedding: Ident) -> (Source, Sink) {
        self.embeddings.insert(embedding.clone());
        NodeBound::graph(embedding)
    }

    fn track_edge(&mut self, span: Span, Sink(from): &Sink, Source(to): &Source) {
        let from_ids: Vec<usize> = match from {
            NodeBound::Literal(idents) => idents
                .iter()
                .map(|id| {
                    *self
                        .node_id_map
                        .get(id)
                        .expect("Missing source node lookup")
                })
                .collect(),
            NodeBound::Graph(graph) => vec![self.graph_as_node(graph)],
            NodeBound::ParallelGroup(group) => vec![self.graph_as_node(group)],
        };

        let to_ids: Vec<usize> = match to {
            NodeBound::Literal(idents) => idents
                .iter()
                .map(|id| {
                    *self
                        .node_id_map
                        .get(id)
                        .expect("Missing target node lookup")
                })
                .collect(),
            NodeBound::Graph(graph) => vec![self.graph_as_node(graph)],
            NodeBound::ParallelGroup(group) => vec![self.graph_as_node(group)],
        };

        for &from_id in &from_ids {
            for &to_id in &to_ids {
                self.compile_graph.add_edge(from_id, to_id);
            }
        }

        if let Err(action_orc_core::GraphError::CycleDetected) = self.compile_graph.sort_ordered() {
            let from_name = match from {
                NodeBound::Literal(idents) => idents
                    .first()
                    .and_then(|id| self.anon_map.get(id))
                    .cloned()
                    .unwrap_or_else(|| "node".to_string()),
                NodeBound::Graph(graph) => format!("#[{graph}]"),
                NodeBound::ParallelGroup(group) => format!("(group: {group})"),
            };

            let to_name = match to {
                NodeBound::Literal(idents) => idents
                    .first()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "node".to_string()),
                NodeBound::Graph(graph) => format!("#[{graph}]",),
                NodeBound::ParallelGroup(group) => format!("(group: {group})"),
            };

            self.errors.push(CodegenError::CircularDependency {
                from: from_name,
                to: to_name,
                span,
            });
        }
    }

    /// Register group or embedded graph as a flat node of compile graph
    /// to resolve circular dependencies at compile-time
    fn graph_as_node(&mut self, ident: &Ident) -> usize {
        if let Some(&id) = self.node_id_map.get(ident) {
            id
        } else {
            let node_id = self.compile_graph.add_node::<DummyMarker>();
            self.node_id_map.insert(ident.clone(), node_id);
            node_id
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
                format!("Task Graph Error: Circular dependency: {from} -> {to}."),
            ),
            CodegenError::VariableCollision { var, span } => syn::Error::new(
                span,
                format!("Task Graph Error: Variable {var} has been already declared."),
            ),
            CodegenError::Syn(syn_err) => syn_err,
        }
    }
}

impl NodeBound {
    fn literal(ident: Ident) -> (Source, Sink) {
        (
            Source(NodeBound::Literal(vec![ident.clone()])),
            Sink(NodeBound::Literal(vec![ident])),
        )
    }

    fn graph(embedding: Ident) -> (Source, Sink) {
        (
            Source(NodeBound::Graph(embedding.clone())),
            Sink(NodeBound::Graph(embedding)),
        )
    }
}

impl IdFactory {
    #[inline]
    fn graph_ident() -> Ident {
        format_ident!("__graph")
    }

    #[inline]
    fn graph_bounds(graph_ident: &Ident) -> (Ident, Ident) {
        (
            format_ident!("_{}_source", graph_ident),
            format_ident!("_{}_sink", graph_ident),
        )
    }

    #[inline]
    fn parallel_group_bounds(group_id: usize) -> (Ident, Ident) {
        (
            format_ident!("group_{}_source", group_id),
            format_ident!("group_{}_sink", group_id),
        )
    }

    #[inline]
    fn anonymous_node(counter: usize) -> Ident {
        format_ident!("anon_{}", counter)
    }
}

impl Default for GraphIdent {
    fn default() -> Self {
        Self(IdFactory::graph_ident())
    }
}
