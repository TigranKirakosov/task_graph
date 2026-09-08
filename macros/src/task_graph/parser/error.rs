use proc_macro2::TokenTree;
#[allow(deprecated)]
use winnow::error::{AddContext, ContextError, ErrorKind, ParserError, StrContext};
use winnow::stream::Stream;

use super::{SpanInfo, current_span};

#[derive(Debug)]
pub(super) struct ParseError<'s> {
    pub(super) span_info: SpanInfo,
    pub(super) inner: ContextError,
    pub(super) input: &'s [TokenTree],
}

impl<'s> ParserError<&'s [TokenTree]> for ParseError<'s> {
    #[inline]
    fn from_input(input: &&'s [TokenTree]) -> Self {
        Self {
            span_info: current_span(input),
            inner: ContextError::from_input(input),
            input,
        }
    }

    #[allow(deprecated)]
    fn from_error_kind(input: &&'s [TokenTree], kind: ErrorKind) -> Self {
        Self {
            span_info: current_span(input),
            inner: ContextError::from_error_kind(input, kind),
            input,
        }
    }

    #[allow(deprecated)]
    fn append(
        mut self,
        input: &&'s [TokenTree],
        token_start: &<&'s [TokenTree] as Stream>::Checkpoint,
        kind: ErrorKind,
    ) -> Self {
        self.span_info = current_span(input);
        self.inner = self.inner.append(input, token_start, kind);
        self.input = input;
        self
    }
}

impl<'s> AddContext<&'s [TokenTree], StrContext> for ParseError<'s> {
    fn add_context(
        mut self,
        input: &&'s [TokenTree],
        token_start: &<&'s [TokenTree] as Stream>::Checkpoint,
        context: StrContext,
    ) -> Self {
        self.inner = self.inner.add_context(input, token_start, context);
        self
    }
}
