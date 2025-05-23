use super::ControlHub;
use crate::framework::prelude::*;

/// The preferred way to build a hub instance in Rust code. Note that the
/// builder cannot be used to instantiate a [`ControlHub`] instance that uses a
/// Control Script; for that see [`ControlHub::from_path`]
pub struct ControlHubBuilder<T: TimingSource> {
    timing: Option<T>,
    ui_controls: Option<UiControls>,
    midi_controls: Option<MidiControls>,
    osc_controls: Option<OscControls>,
    audio_controls: Option<AudioControls>,
}

impl<T: TimingSource> Default for ControlHubBuilder<T> {
    fn default() -> Self {
        Self {
            timing: None,
            ui_controls: None,
            midi_controls: None,
            osc_controls: None,
            audio_controls: None,
        }
    }
}

impl<T: TimingSource> ControlHubBuilder<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timing(mut self, timing: T) -> Self {
        self.timing = Some(timing);
        self
    }

    pub fn ui_controls(mut self, controls: UiControls) -> Self {
        self.ui_controls = Some(controls);
        self
    }

    fn ensure_ui_controls(&mut self) -> &mut UiControls {
        if self.ui_controls.is_none() {
            self.ui_controls = Some(UiControls::default());
        }
        self.ui_controls.as_mut().unwrap()
    }

    pub fn ui(mut self, control: UiControlConfig) -> Self {
        let clone = control.clone();
        let name = clone.name();
        self.ensure_ui_controls().add(name, control);
        self
    }

    pub fn checkbox(
        self,
        name: &str,
        value: bool,
        disabled: DisabledFn,
    ) -> Self {
        self.ui(UiControlConfig::Checkbox {
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
        self.ui(UiControlConfig::Select {
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
        self.ui(UiControlConfig::Slider {
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
        self.ui(UiControlConfig::Separator { name: uuid_5() })
    }

    pub fn midi_controls(mut self, midi_controls: MidiControls) -> Self {
        self.midi_controls = Some(midi_controls);
        self
    }

    fn ensure_midi_controls(&mut self) -> &mut MidiControls {
        if self.midi_controls.is_none() {
            self.midi_controls = Some(MidiControls::default());
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

    pub fn hrcc(mut self, hrcc: bool) -> Self {
        self.ensure_midi_controls().hrcc = hrcc;
        self
    }

    pub fn osc_controls(mut self, osc_controls: OscControls) -> Self {
        self.osc_controls = Some(osc_controls);
        self
    }

    fn ensure_osc_controls(&mut self) -> &mut OscControls {
        if self.osc_controls.is_none() {
            self.osc_controls = Some(OscControls::default());
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
            self.audio_controls =
                Some(AudioControls::new(default_buffer_processor));
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

    pub fn build(self) -> ControlHub<T> {
        let mut c = ControlHub::new(None, self.timing.unwrap());

        if let Some(controls) = self.ui_controls {
            c.ui_controls = controls;
        }

        if let Some(midi_controls) = self.midi_controls {
            c.midi_controls = midi_controls;
            if let Err(e) = c.midi_controls.start() {
                error!("Unable to start midi_controls: {}", e);
            }
        }

        if let Some(osc_controls) = self.osc_controls {
            c.osc_controls = osc_controls;
            if let Err(e) = c.osc_controls.start() {
                error!("Unable to start osc_controls: {}", e);
            }
        }

        if let Some(audio_controls) = self.audio_controls {
            c.audio_controls = audio_controls;
            if let Err(e) = c.audio_controls.start() {
                error!("Unable to start audio_controls: {}", e);
            }
        }

        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_script_builder() {
        let controls: ControlHub<ManualTiming> = ControlHubBuilder::new()
            .timing(ManualTiming::new(Bpm::new(134.0)))
            .ui_controls(UiControlBuilder::new().slider_n("foo", 0.5).build())
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
                            value: 11.0,
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
        let controls = ControlHubBuilder::new()
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
                    value: 11.0,
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
