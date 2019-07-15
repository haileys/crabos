// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use core::borrow::Borrow;
use core::cmp::Ordering;

use super::node::{Handle, NodeRef, marker, ForceResult::*};

use crate::glue::GlobalAlloc;

use SearchResult::*;

pub enum SearchResult<BorrowType, K, V, FoundType, GoDownType, Allocator> {
    Found(Handle<NodeRef<BorrowType, K, V, FoundType, Allocator>, marker::KV, Allocator>),
    GoDown(Handle<NodeRef<BorrowType, K, V, GoDownType, Allocator>, marker::Edge, Allocator>)
}

pub fn search_tree<BorrowType, K, V, Q: ?Sized, Allocator: GlobalAlloc>(
    mut node: NodeRef<BorrowType, K, V, marker::LeafOrInternal, Allocator>,
    key: &Q
) -> SearchResult<BorrowType, K, V, marker::LeafOrInternal, marker::Leaf, Allocator>
        where Q: Ord, K: Borrow<Q> {

    loop {
        match search_node(node, key) {
            Found(handle) => return Found(handle),
            GoDown(handle) => match handle.force() {
                Leaf(leaf) => return GoDown(leaf),
                Internal(internal) => {
                    node = internal.descend();
                    continue;
                }
            }
        }
    }
}

pub fn search_node<BorrowType, K, V, Type, Q: ?Sized, Allocator: GlobalAlloc>(
    node: NodeRef<BorrowType, K, V, Type, Allocator>,
    key: &Q
) -> SearchResult<BorrowType, K, V, Type, Type, Allocator>
        where Q: Ord, K: Borrow<Q> {

    match search_linear(&node, key) {
        (idx, true) => Found(
            Handle::new_kv(node, idx)
        ),
        (idx, false) => SearchResult::GoDown(
            Handle::new_edge(node, idx)
        )
    }
}

pub fn search_linear<BorrowType, K, V, Type, Q: ?Sized, Allocator: GlobalAlloc>(
    node: &NodeRef<BorrowType, K, V, Type, Allocator>,
    key: &Q
) -> (usize, bool)
        where Q: Ord, K: Borrow<Q> {

    for (i, k) in node.keys().iter().enumerate() {
        match key.cmp(k.borrow()) {
            Ordering::Greater => {},
            Ordering::Equal => return (i, true),
            Ordering::Less => return (i, false)
        }
    }
    (node.keys().len(), false)
}
