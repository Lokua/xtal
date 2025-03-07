use super::ControlScript;
use crate::framework::prelude::*;

pub struct ControlScriptBuilder<T: TimingSource> {
    timing: Option<T>,
    controls: Option<Controls>,
    midi_controls: Option<MidiControls>,
    osc_controls: Option<OscControls>,
    audio_controls: Option<AudioControls>,
}

impl<T: TimingSource> ControlScriptBuilder<T> {
    pub fn new() -> Self {
        Self {
            timing: None,
            controls: None,
            midi_controls: None,
            osc_controls: None,
            audio_controls: None,
        }
    }

    pub fn timing(mut self, timing: T) -> Self {
        self.timing = Some(timing);
        self
    }

    pub fn controls(mut self, controls: Controls) -> Self {
        self.controls = Some(controls);
        self
    }

    pub fn midi_controls(mut self, midi_controls: MidiControls) -> Self {
        self.midi_controls = Some(midi_controls);
        self
    }

    pub fn osc_controls(mut self, osc_controls: OscControls) -> Self {
        self.osc_controls = Some(osc_controls);
        self
    }

    pub fn audio_controls(mut self, audio_controls: AudioControls) -> Self {
        self.audio_controls = Some(audio_controls);
        self
    }

    pub fn build(self) -> ControlScript<T> {
        let mut c = ControlScript::new(None, self.timing.unwrap());

        if let Some(controls) = self.controls {
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
            .controls(Controls::new(vec![Control::slide("foo", 0.5)]))
            .osc_controls(OscControlBuilder::new().control("bar", 22.0).build())
            .midi_controls(
                MidiControlBuilder::new()
                    .control("baz", (0, 0), 0.66)
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
}
