use std::any::Any;
use std::error::Error;

use super::prelude::*;
use crate::runtime::storage::load_controls;

/// Shared trait to enable sketches to use either `Controls` or `ControlScript`
/// instances via a single `Model.controls` struct field and have them
/// automatically show up in the UI in either case.
pub trait ControlProvider {
    fn controls_mut(&mut self) -> &mut Controls;
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
    fn osc_controls(&self) -> Option<OscControls>;
    fn controls(&self) -> Option<Controls>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn load_controls(
        &mut self,
        sketch_name: &str,
    ) -> Option<Result<(), Box<dyn Error>>> {
        if !self.is_control_script() {
            return None;
        }

        let cs = self.as_any_mut();

        if let Some(c) = cs.downcast_mut::<ControlScript<Timing>>() {
            return Some(load_controls::<Timing>(sketch_name, c));
        }

        if let Some(c) = cs.downcast_mut::<ControlScript<FrameTiming>>() {
            return Some(load_controls::<FrameTiming>(sketch_name, c));
        }

        if let Some(c) = cs.downcast_mut::<ControlScript<OscTransportTiming>>()
        {
            return Some(load_controls::<OscTransportTiming>(sketch_name, c));
        }

        if let Some(c) = cs.downcast_mut::<ControlScript<MidiSongTiming>>() {
            return Some(load_controls::<MidiSongTiming>(sketch_name, c));
        }

        if let Some(c) = cs.downcast_mut::<ControlScript<HybridTiming>>() {
            return Some(load_controls::<HybridTiming>(sketch_name, c));
        }

        if let Some(c) = cs.downcast_mut::<ControlScript<ManualTiming>>() {
            return Some(load_controls::<ManualTiming>(sketch_name, c));
        }

        None
    }
}

impl ControlProvider for Controls {
    fn controls_mut(&mut self) -> &mut Controls {
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
    fn osc_controls(&self) -> Option<OscControls> {
        None
    }
    fn controls(&self) -> Option<Controls> {
        Some(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<T: TimingSource + 'static> ControlProvider for ControlScript<T> {
    fn controls_mut(&mut self) -> &mut Controls {
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
    fn osc_controls(&self) -> Option<OscControls> {
        Some(self.osc_controls.clone())
    }
    fn controls(&self) -> Option<Controls> {
        Some(self.controls.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
