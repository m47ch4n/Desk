use std::collections::HashMap;

use ids::LinkName;
use serde::{Deserialize, Serialize};
use types::{Effect, Type};

use crate::{block::BlockId, mir::ControlFlowGraphId, var::VarId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StmtBind<T = Stmt> {
    pub var: VarId,
    pub stmt: T,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stmt {
    Const(Const),
    Product(Vec<VarId>),
    Vector(Vec<VarId>),
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
    /// Used when cast is required such as `* A, B` to `A` or `A` to `+ A, B`.
    /// An implementation of MIR generator may generate redundant `Cast` stmt.
    Cast(VarId),
    Parameter,
    Recursion,
    Link(LinkName),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FnRef {
    Link(Type),
    Closure {
        mir: ControlFlowGraphId,
        /// Caputerd variables
        captured: Vec<VarId>,
        /// Used to create an effectful expression
        handlers: HashMap<Effect, VarId>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Const {
    Int(i64),
    Rational(i64, i64),
    Float(f64),
    String(String),
}

// Const::Float should not be NaN
impl Eq for Const {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchCase<T = Type> {
    pub ty: T,
    pub next: BlockId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terminator<T = Type> {
    Return(VarId),
    Match {
        var: VarId,
        cases: Vec<MatchCase<T>>,
    },
    Goto(BlockId),
}

pub type LinkId = ids::LinkId<Type>;
