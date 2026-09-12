use std::marker::PhantomData;

use crate::{Graph, Marker, Meta};

pub enum GraphEntry<'a> {
    Node(Meta),
    Graph(&'a Graph),
}

pub trait AsGraphEntry<'a> {
    fn as_entry(this: Self) -> GraphEntry<'a>;
}

pub struct Tag<T>(PhantomData<T>);

impl<'a, T: Marker> AsGraphEntry<'a> for Tag<T> {
    fn as_entry(_this: Self) -> GraphEntry<'a> {
        GraphEntry::Node(T::meta())
    }
}

impl<'a> AsGraphEntry<'a> for &'a Graph {
    fn as_entry(this: Self) -> GraphEntry<'a> {
        GraphEntry::Graph(this)
    }
}
