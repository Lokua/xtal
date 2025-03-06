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
    fn is_control_script(&self) -> bool;
    fn take_snapshot(&mut self, id: &str);
    fn recall_snapshot(&mut self, id: &str);
    fn delete_snapshot(&mut self, id: &str);
    fn clear_snapshots(&mut self);
    fn midi_controls(&self) -> Option<MidiControls>;
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

    fn is_control_script(&self) -> bool {
        false
    }

    fn take_snapshot(&mut self, _id: &str) {
        warn!("Controls doesn't have snapshots");
    }
    fn recall_snapshot(&mut self, _id: &str) {
        warn!("Controls doesn't have snapshots");
    }
    fn delete_snapshot(&mut self, _id: &str) {
        warn!("Controls doesn't have snapshots");
    }
    fn clear_snapshots(&mut self) {
        warn!("Controls doesn't have snapshots");
    }

    fn midi_controls(&self) -> Option<MidiControls> {
        None
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

    fn is_control_script(&self) -> bool {
        true
    }

    fn take_snapshot(&mut self, id: &str) {
        self.take_snapshot(id);
    }
    fn recall_snapshot(&mut self, id: &str) {
        self.recall_snapshot(id);
    }
    fn delete_snapshot(&mut self, id: &str) {
        self.delete_snapshot(id);
    }
    fn clear_snapshots(&mut self) {
        self.clear_snapshots();
    }

    fn midi_controls(&self) -> Option<MidiControls> {
        Some(self.midi_controls.clone())
    }
}
