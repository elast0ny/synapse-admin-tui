pub mod synapse;
use std::borrow::Cow;

pub use synapse::Synapse;

use crate::editable::EditableWidget;

pub enum Backend {
    Synapse(Synapse),
}
pub struct MissingInfo<'b> {
    /// A list of title & input boxes that need filling
    pub fields: Vec<(Cow<'b, str>, &'b mut dyn EditableWidget)>,
    /// Extra message (to indicate what the user should fill, errors, etc...)
    pub message: Cow<'b, str>,
}
impl<'b> Default for MissingInfo<'b> {
    fn default() -> Self {
        Self {
            fields: Default::default(),
            message: "".into(),
        }
    }
}
