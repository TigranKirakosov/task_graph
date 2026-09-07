use proc_macro2::TokenTree;
use winnow::{ModalResult, Parser, combinator::eof};

use super::{ParseError, SpanInfo};

pub(super) trait TokenStreamParseExt {
    fn parse_nested<'a, O, F>(
        self,
        group_span: SpanInfo,
        outer_input: &'a [TokenTree],
        parser: F,
    ) -> ModalResult<O, ParseError<'a>>
    where
        F: for<'b> FnMut(&mut &'b [TokenTree]) -> ModalResult<O, ParseError<'b>>;
}

impl TokenStreamParseExt for proc_macro2::TokenStream {
    fn parse_nested<'a, O, F>(
        self,
        group_span: SpanInfo,
        outer_input: &'a [TokenTree],
        mut parser: F,
    ) -> ModalResult<O, ParseError<'a>>
    where
        F: for<'b> FnMut(&mut &'b [TokenTree]) -> ModalResult<O, ParseError<'b>>,
    {
        let tokens: Vec<TokenTree> = self.into_iter().collect();
        let mut tokens_slice = tokens.as_slice();

        let output = parser.parse_next(&mut tokens_slice).map_err(|err_mode| {
            err_mode.map(|err| ParseError {
                span_info: err.span_info,
                inner: err.inner,
                input: outer_input,
            })
        })?;

        eof::<_, ParseError>
            .parse_next(&mut tokens_slice)
            .map_err(|err_mode| {
                err_mode.map(|err| ParseError {
                    span_info: if err.span_info.at_call_site {
                        group_span
                    } else {
                        err.span_info
                    },
                    inner: err.inner,
                    input: outer_input,
                })
            })?;

        Ok(output)
    }
}
