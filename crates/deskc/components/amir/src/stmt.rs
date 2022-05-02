use std::collections::HashMap;

use link::LinkName;
use types::{Effect, Type};

use crate::{amir::AmirId, block::BlockId, var::VarId};

#[cfg_attr(feature = "withserde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct StmtBind<T = AStmt> {
    pub var: VarId,
    pub stmt: T,
}

#[cfg_attr(feature = "withserde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum AStmt {
    Const(Const),
    Product(Vec<VarId>),
    Array(Vec<VarId>),
    Set(Vec<VarId>),
    Fn(FnRef),
    Perform(VarId),
    MatchResult(VarId),
    Apply {
        function: VarId,
        arguments: Vec<VarId>,
    },
    Op {
        op: Op,
        operands: Vec<VarId>,
    },
    Cast(VarId),
    Parameter,
    Recursion,
    Link(LinkName),
}

#[cfg_attr(feature = "withserde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum FnRef {
    Link(Type),
    Closure {
        amir: AmirId,
        /// Caputerd variables
        captured: Vec<VarId>,
        /// Used to create an effectful expression
        handlers: HashMap<Effect, VarId>,
    },
}

#[cfg_attr(feature = "withserde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum Const {
    Int(i64),
    Rational(i64, i64),
    Float(f64),
    String(String),
}

#[cfg_attr(feature = "withserde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Mod,
    Pow,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    Not,
    Neg,
    Pos,
    Shl,
    Shr,
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
}

#[cfg_attr(feature = "withserde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MatchCase<T = Type> {
    pub ty: T,
    pub next: BlockId,
}

#[cfg_attr(feature = "withserde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum ATerminator<T = Type> {
    Return(VarId),
    Match {
        var: VarId,
        cases: Vec<MatchCase<T>>,
    },
    Goto(BlockId),
}

#[cfg_attr(feature = "withserde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct ALink<T = Type> {
    pub name: LinkName,
    pub ty: T,
}