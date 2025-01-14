#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProcessorId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessorAttachment {
    Attached(ProcessorId),
    Detached,
}

impl Default for ProcessorAttachment {
    fn default() -> Self {
        Self::Detached
    }
}
