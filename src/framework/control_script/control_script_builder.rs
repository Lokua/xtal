use super::ControlScript;
use crate::config::AUDIO_DEVICE_SAMPLE_RATE;
use crate::framework::{frame_controller, prelude::*};

pub struct ControlScriptBuilder<T: TimingSource> {
    timing: Option<T>,
    ui_controls: Option<Controls>,
    midi_controls: Option<MidiControls>,
    osc_controls: Option<OscControls>,
    audio_controls: Option<AudioControls>,
}

impl<T: TimingSource> ControlScriptBuilder<T> {
    pub fn new() -> Self {
        Self {
            timing: None,
            ui_controls: None,
            midi_controls: None,
            osc_controls: None,
            audio_controls: None,
        }
    }

    pub fn timing(mut self, timing: T) -> Self {
        self.timing = Some(timing);
        self
    }

    pub fn ui_controls(mut self, controls: Controls) -> Self {
        self.ui_controls = Some(controls);
        self
    }

    fn ensure_ui_controls(&mut self) -> &mut Controls {
        if self.ui_controls.is_none() {
            self.ui_controls = Some(Controls::with_previous(vec![]));
        }
        self.ui_controls.as_mut().unwrap()
    }

    pub fn ui(mut self, control: Control) -> Self {
        self.ensure_ui_controls().add(control);
        self
    }

    pub fn button(self, name: &str, disabled: DisabledFn) -> Self {
        self.ui(Control::Button {
            name: name.to_string(),
            disabled,
        })
    }

    pub fn checkbox(
        self,
        name: &str,
        value: bool,
        disabled: DisabledFn,
    ) -> Self {
        self.ui(Control::Checkbox {
            name: name.to_string(),
            value,
            disabled,
        })
    }

    pub fn select<S>(
        self,
        name: &str,
        value: &str,
        options: &[S],
        disabled: DisabledFn,
    ) -> Self
    where
        S: AsRef<str>,
    {
        self.ui(Control::Select {
            name: name.into(),
            value: value.into(),
            options: options.iter().map(|s| s.as_ref().to_string()).collect(),
            disabled,
        })
    }

    pub fn slider(
        self,
        name: &str,
        value: f32,
        range: (f32, f32),
        step: f32,
        disabled: DisabledFn,
    ) -> Self {
        self.ui(Control::Slider {
            name: name.to_string(),
            value,
            min: range.0,
            max: range.1,
            step,
            disabled,
        })
    }

    pub fn slider_n(self, name: &str, value: f32) -> Self {
        self.slider(name, value, (0.0, 1.0), 0.0001, None)
    }

    pub fn separator(self) -> Self {
        self.ui(Control::Separator {})
    }

    pub fn dynamic_separator(self, name: &str) -> Self {
        self.ui(Control::DynamicSeparator {
            name: name.to_string(),
        })
    }

    pub fn midi_controls(mut self, midi_controls: MidiControls) -> Self {
        self.midi_controls = Some(midi_controls);
        self
    }

    fn ensure_midi_controls(&mut self) -> &mut MidiControls {
        if self.midi_controls.is_none() {
            self.midi_controls = Some(MidiControls::new());
        }
        self.midi_controls.as_mut().unwrap()
    }

    pub fn midi(
        mut self,
        name: &str,
        midi: (u8, u8),
        range: (f32, f32),
        default: f32,
    ) -> Self {
        self.ensure_midi_controls()
            .add(name, MidiControlConfig::new(midi, range, default));
        self
    }

    pub fn midi_n(self, name: &str, midi: (u8, u8)) -> Self {
        self.midi(name, midi, (0.0, 1.0), 0.0)
    }

    pub fn osc_controls(mut self, osc_controls: OscControls) -> Self {
        self.osc_controls = Some(osc_controls);
        self
    }

    fn ensure_osc_controls(&mut self) -> &mut OscControls {
        if self.osc_controls.is_none() {
            self.osc_controls = Some(OscControls::new());
        }
        self.osc_controls.as_mut().unwrap()
    }

    pub fn osc(
        mut self,
        address: &str,
        range: (f32, f32),
        default: f32,
    ) -> Self {
        self.ensure_osc_controls()
            .add(address, OscControlConfig::new(address, range, default));
        self
    }

    pub fn osc_n(self, address: &str, default: f32) -> Self {
        self.osc(address, (0.0, 1.0), default)
    }

    pub fn audio_controls(mut self, audio_controls: AudioControls) -> Self {
        self.audio_controls = Some(audio_controls);
        self
    }

    fn ensure_audio_controls(&mut self) -> &mut AudioControls {
        if self.audio_controls.is_none() {
            self.audio_controls = Some(AudioControls::new(
                frame_controller::fps(),
                AUDIO_DEVICE_SAMPLE_RATE,
                default_buffer_processor,
            ));
        }
        self.audio_controls.as_mut().unwrap()
    }

    pub fn buffer_processor(
        mut self,
        buffer_processor: BufferProcessor,
    ) -> Self {
        self.ensure_audio_controls()
            .set_buffer_processor(buffer_processor);
        self
    }

    pub fn audio(mut self, name: &str, config: AudioControlConfig) -> Self {
        self.ensure_audio_controls().add(name, config);
        self
    }

    pub fn build(self) -> ControlScript<T> {
        let mut c = ControlScript::new(None, self.timing.unwrap());

        if let Some(controls) = self.ui_controls {
            c.controls = controls;
        }

        if let Some(midi_controls) = self.midi_controls {
            c.midi_controls = midi_controls;
        }

        if let Some(osc_controls) = self.osc_controls {
            c.osc_controls = osc_controls;
        }

        if let Some(audio_controls) = self.audio_controls {
            c.audio_controls = audio_controls;
        }

        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_script_builder() {
        let controls: ControlScript<ManualTiming> = ControlScriptBuilder::new()
            .timing(ManualTiming::new(Bpm::new(134.0)))
            .ui_controls(Controls::new(vec![Control::slide("foo", 0.5)]))
            .osc_controls(
                OscControlBuilder::new().control_n("bar", 22.0).build(),
            )
            .midi_controls(
                MidiControlBuilder::new()
                    .control_n("baz", (0, 0), 0.66)
                    .build(),
            )
            .audio_controls(
                AudioControlBuilder::new()
                    .control_from_config(
                        "qux",
                        AudioControlConfig {
                            channel: 0,
                            slew_limiter: SlewLimiter::default(),
                            pre_emphasis: 0.0,
                            detect: 0.0,
                            range: (0.0, 1.0),
                            default: 11.0,
                        },
                    )
                    .build(),
            )
            .build();

        assert_eq!(controls.get("foo"), 0.5);
        assert_eq!(controls.get("bar"), 22.0);
        assert_eq!(controls.get("baz"), 0.66);

        // Buffer gets overridden immediately so not really testable
        // assert_eq!(controls.get("qux"), 11.0);
    }

    #[test]
    fn test_control_script_builder_helpers() {
        let controls: ControlScript<ManualTiming> = ControlScriptBuilder::new()
            .timing(ManualTiming::new(Bpm::new(134.0)))
            .slider_n("foo", 0.5)
            .osc_n("bar", 22.0)
            .midi("baz", (0, 0), (0.0, 1.0), 0.66)
            .audio(
                "qux",
                AudioControlConfig {
                    channel: 0,
                    slew_limiter: SlewLimiter::default(),
                    pre_emphasis: 0.0,
                    detect: 0.0,
                    range: (0.0, 1.0),
                    default: 11.0,
                },
            )
            .build();

        assert_eq!(controls.get("foo"), 0.5);
        assert_eq!(controls.get("bar"), 22.0);
        assert_eq!(controls.get("baz"), 0.66);

        // Buffer gets overridden immediately so not really testable
        // assert_eq!(controls.get("qux"), 11.0);
    }
}
