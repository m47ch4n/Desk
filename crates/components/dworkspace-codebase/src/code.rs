use std::sync::Arc;

use ast::{expr::Expr, span::WithSpan};

/// A unit of code in a codebase.
///
/// Uses Arc for cheap cloning
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Code {
    SourceCode {
        syntax: SyntaxKind,
        source: Arc<String>,
    },
    Ast(Arc<WithSpan<Expr>>),
}

// Some syntax are not supported yet.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    Hacker,
    TypeScriptLike,
    OCamlLike,
    RustLike,
}
