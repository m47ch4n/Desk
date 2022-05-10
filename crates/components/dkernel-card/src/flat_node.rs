use std::collections::HashMap;

use deskc_ids::NodeId;
use hir::expr::Expr;
use types::Type;

use crate::content::Content;

pub type Children = Vec<NodeRef>;
pub type Attributes = HashMap<Type, Expr>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatNode {
    /// The content of the node.
    pub content: Content,
    pub children: Children,
    pub attributes: Attributes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeRef {
    Hole,
    Node(NodeId),
}
