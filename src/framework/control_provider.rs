use super::prelude::*;

pub trait ControlProvider {
    fn as_controls(&mut self) -> &mut Controls;
    fn get_controls(&self) -> &Vec<Control>;
    fn update_value(&mut self, name: &str, value: ControlValue);
    fn to_serialized(&self) -> SerializedControls;
}

impl ControlProvider for Controls {
    fn as_controls(&mut self) -> &mut Controls {
        self
    }

    fn get_controls(&self) -> &Vec<Control> {
        Controls::get_controls(self)
    }

    fn update_value(&mut self, name: &str, value: ControlValue) {
        Controls::update_value(self, name, value)
    }

    fn to_serialized(&self) -> SerializedControls {
        Controls::to_serialized(self)
    }
}

impl<T: TimingSource> ControlProvider for ControlScript<T> {
    fn as_controls(&mut self) -> &mut Controls {
        &mut self.controls
    }

    fn get_controls(&self) -> &Vec<Control> {
        self.controls.get_controls()
    }

    fn update_value(&mut self, name: &str, value: ControlValue) {
        self.controls.update_value(name, value)
    }

    fn to_serialized(&self) -> SerializedControls {
        self.controls.to_serialized()
    }
}
