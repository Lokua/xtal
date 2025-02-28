use super::prelude::*;

/// Shared trait to enable sketches to use either `Controls` or `ControlScript`
/// instances via a single `Model.controls` struct field and have them
/// automatically show up in the UI in either case.
pub trait ControlProvider {
    fn as_controls_mut(&mut self) -> &mut Controls;
    fn items(&self) -> &Vec<Control>;
    fn items_mut(&mut self) -> &mut Vec<Control>;
    fn update_value(&mut self, name: &str, value: ControlValue);
    fn to_serialized(&self) -> SerializedControls;
}

impl ControlProvider for Controls {
    fn as_controls_mut(&mut self) -> &mut Controls {
        self
    }

    fn items(&self) -> &Vec<Control> {
        Controls::items(self)
    }

    fn items_mut(&mut self) -> &mut Vec<Control> {
        self.items_mut()
    }

    fn update_value(&mut self, name: &str, value: ControlValue) {
        Controls::update_value(self, name, value)
    }

    fn to_serialized(&self) -> SerializedControls {
        Controls::to_serialized(self)
    }
}

impl<T: TimingSource> ControlProvider for ControlScript<T> {
    fn as_controls_mut(&mut self) -> &mut Controls {
        &mut self.controls
    }

    fn items(&self) -> &Vec<Control> {
        self.controls.items()
    }

    fn items_mut(&mut self) -> &mut Vec<Control> {
        self.controls.items_mut()
    }

    fn update_value(&mut self, name: &str, value: ControlValue) {
        self.controls.update_value(name, value)
    }

    fn to_serialized(&self) -> SerializedControls {
        self.controls.to_serialized()
    }
}
