use std::any::Any;

use crate::framework::prelude::*;

/// Type erasure trait that enables object-safe access to generic
/// `ControlHub<T>` instances. This enables boxed sketches to expose their
/// controls to the UI and runtime systems without breaking object safety rules.
/// It serves as the interface layer between the type-parameterized
/// `ControlHub<T>` and the object-safe `SketchAll` trait used in the
/// registry.
pub trait ControlProvider {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: TimingSource + 'static> ControlProvider for ControlHub<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
