use crate::task_graph::parser::{
    combinators::enclosed,
    error::{SpanInfo, current_span},
    ext::TokenStreamParseExt,
};

use super::ast::*;
use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use winnow::{
    ModalResult, Parser,
    combinator::{alt, preceded, repeat, separated},
    error::{ErrMode, ParserError, StrContext, StrContextValue},
    stream::Stream,
};

use combinators::*;
use error::ParseError;

mod combinators;
mod error;
mod ext;

#[cfg(test)]
mod tests;

pub(super) fn parse(stream: TokenStream2) -> Result<TaskGraphAst, syn::Error> {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut input = tokens.as_slice();

    match separated(0.., graph, punct(';')).parse_next(&mut input) {
        Ok(graphs) => Ok(TaskGraphAst { graphs }),
        Err(err_mode) => {
            let parse_err = err_mode
                .into_inner()
                .expect("Parser failed or encountered incomplete input state");

            Err(syn::Error::new(
                parse_err.span_info.span,
                format!("Failed to parse task schedule: {}", parse_err.inner),
            ))
        }
    }
}

/// (a: A | b: B) -> C -> [d];
fn graph<'a>(input: &mut &'a [TokenTree]) -> ModalResult<Graph, ParseError<'a>> {
    let entry = node_expr.parse_next(input)?;
    let conns = repeat(0.., preceded(arrow, node_expr)).parse_next(input)?;
    Ok(Graph { entry, conns })
}

fn node_expr<'a>(input: &mut &'a [TokenTree]) -> ModalResult<NodeExpr, ParseError<'a>> {
    let expr = alt((binding, decl, group)).parse_next(input)?;
    Ok(expr)
}

/// 1) a: A
/// 2) A
fn decl<'a>(input: &mut &'a [TokenTree]) -> ModalResult<NodeExpr, ParseError<'a>> {
    if let Some(TokenTree::Group(g)) = input.first() {
        if g.delimiter() == Delimiter::Bracket || g.delimiter() == Delimiter::Parenthesis {
            return Err(ErrMode::Backtrack(ParseError::from_input(input)));
        }
    }

    let task = alt((
        // var: scenario::Entering<Dungeon>
        (ident, punct(':'), type_path).map(|(var, _, typ)| Task {
            var: Some(var),
            typ,
        }),
        // scenario::Entering<Dungeon>
        type_path.map(|typ| Task { var: None, typ }),
    ))
    .parse_next(input)?;

    Ok(NodeExpr::Declaration(task))
}

/// [var]
fn binding<'a>(input: &mut &'a [TokenTree]) -> ModalResult<NodeExpr, ParseError<'a>> {
    let expr = enclosed(Delimiter::Bracket, ident, "var binding")
        .map(NodeExpr::Binding)
        .parse_next(input)?;

    Ok(expr)
}

/// 1) (A | B | C)
/// 2) (A, B, C)
fn group<'a>(input: &mut &'a [TokenTree]) -> ModalResult<NodeExpr, ParseError<'a>> {
    let expr = enclosed(
        Delimiter::Parenthesis,
        |i| alt((parallel_block, sequence_block)).parse_next(i),
        "node group",
    )
    .map(NodeExpr::Group)
    .parse_next(input)?;

    Ok(expr)
}

fn parallel_block<'a>(input: &mut &'a [TokenTree]) -> ModalResult<GroupBlock, ParseError<'a>> {
    let checkpoint = input.checkpoint();
    match separated(2.., graph, punct('|')).parse_next(input) {
        Ok(graphs) => {
            let block = GroupBlock {
                mode: SchedulingMode::Parallel,
                graphs,
            };

            Ok(block)
        }
        Err(err) => {
            input.reset(&checkpoint);
            Err(err)
        }
    }
}

fn sequence_block<'a>(input: &mut &'a [TokenTree]) -> ModalResult<GroupBlock, ParseError<'a>> {
    let checkpoint = input.checkpoint();
    match separated(2.., graph, punct(',')).parse_next(input) {
        Ok(graphs) => {
            let block = GroupBlock {
                mode: SchedulingMode::Sequence,
                graphs,
            };

            Ok(block)
        }
        Err(err) => {
            input.reset(&checkpoint);
            Err(err)
        }
    }
}

/// ->
fn arrow<'a>(input: &mut &'a [TokenTree]) -> ModalResult<(), ParseError<'a>> {
    (punct('-'), punct('>'))
        .context(StrContext::Expected(StrContextValue::Description(
            "right arrow (->)",
        )))
        .void()
        .parse_next(input)
}
