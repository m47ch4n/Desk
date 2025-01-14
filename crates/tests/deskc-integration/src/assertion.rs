use dprocess::value::Value;
use serde::{Deserialize, Serialize};
use types::Type;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Assertion {
    RunSuccess { result: Value },
    Typed(Vec<(usize, Type)>),
}
