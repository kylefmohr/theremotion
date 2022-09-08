use std::ops::RangeInclusive;

use faust_state::{Node, RangedInput, StateHandle, WidgetType};
use staff::midi::MidiNote;

use crate::settings::Settings;

/// Ability to exchange with the DSP state
pub trait ControlTrait {
    /// Set the current state to the DSP
    fn send(&mut self, state: &mut StateHandle);
}

/// DSP controls
#[derive(Debug, Clone)]
pub struct Controls {
    /// Midi note
    pub note1: NoteControl,
    /// Volume of the main voice
    pub vol1: Control,
    /// Chord notes
    pub lead: [NoteControl2; 3],
    /// Filter cutoff
    pub cutoff_note: Control,
    /// Filter resonnance
    pub resonance: Control,
    /// Supersaw volume
    pub supersaw: Control,
    /// Supersaw detune
    pub detune: Control,
    /// Guitar pluck
    pub pluck: BoolControl,
    /// Guitare note
    pub pluck_note: Control,
    /// Guitar pluck gain
    pub pluck_gain: Control,
    /// Guitar pluck damping
    pub pluck_damping: Control,
    /// Drone volume
    pub drone_volume: Control,
    /// Drone note
    pub drone_note: Control,

    /// Warning message
    pub warning: Option<String>,
    /// Error message
    pub error: Option<String>,
}

impl ControlTrait for Controls {
    fn send(&mut self, state: &mut StateHandle) {
        self.note1.send(state);
        self.vol1.send(state);
        for note in &mut self.lead {
            note.note.send(state);
            note.volume.send(state);
        }
        self.cutoff_note.send(state);
        self.resonance.send(state);
        self.supersaw.send(state);
        self.detune.send(state);
        self.pluck.send(state);
        self.pluck_note.send(state);
        self.pluck_gain.send(state);
        self.pluck_damping.send(state);
        self.drone_volume.send(state);
        self.drone_note.send(state);
        state.send();
    }
}

impl From<&StateHandle> for Controls {
    fn from(state: &StateHandle) -> Self {
        Self {
            note1: state.node_by_path("lead/note1").unwrap().into(),
            vol1: state.node_by_path("lead/vol1").unwrap().into(),
            lead: [
                (
                    state.node_by_path("lead/note2").unwrap(),
                    state.node_by_path("lead/vol2").unwrap(),
                )
                    .into(),
                (
                    state.node_by_path("lead/note3").unwrap(),
                    state.node_by_path("lead/vol3").unwrap(),
                )
                    .into(),
                (
                    state.node_by_path("lead/note4").unwrap(),
                    state.node_by_path("lead/vol4").unwrap(),
                )
                    .into(),
            ],
            cutoff_note: state
                .node_by_path("lead/filter/cutoff_note")
                .unwrap()
                .into(),
            resonance: state.node_by_path("lead/filter/res").unwrap().into(),
            supersaw: state.node_by_path("lead/supersaw/volume").unwrap().into(),
            detune: state.node_by_path("lead/supersaw/detune").unwrap().into(),
            pluck: state.node_by_path("pluck/gate").unwrap().into(),
            pluck_note: state.node_by_path("pluck/note").unwrap().into(),
            pluck_gain: state.node_by_path("pluck/gain").unwrap().into(),
            pluck_damping: state.node_by_path("pluck/damping").unwrap().into(),
            drone_volume: state.node_by_path("drone/volume").unwrap().into(),
            drone_note: state.node_by_path("drone/note").unwrap().into(),
            warning: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Control {
    /// Current value of the control in the DSP
    pub value: f32,

    /// DSP metadata
    pub input: RangedInput,

    /// Name for the DSP
    pub path: String,
}

impl ControlTrait for Control {
    fn send(&mut self, state: &mut StateHandle) {
        state.set_by_path(&self.path, self.value).unwrap();
    }
}

impl Control {
    pub fn set_scaled(&mut self, value: f32, value_range: RangeInclusive<f32>) {
        self.value = convert_range(value, value_range, &self.input.range);
    }
}

impl From<&Node> for Control {
    fn from(node: &Node) -> Self {
        let input = match node.widget_type() {
            WidgetType::VerticalSlider(input) => input,
            WidgetType::HorizontalSlider(input) => input,
            WidgetType::NumEntry(input) => input,
            _ => panic!("The parameter {} is not a ranged input.", node.path()),
        };
        Self {
            value: input.init,
            input: input.clone(),
            path: node.path(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoolControl {
    /// On-off state
    pub value: bool,

    /// Name for the DSP
    pub path: String,
}

impl ControlTrait for BoolControl {
    fn send(&mut self, state: &mut StateHandle) {
        state
            .set_by_path(&self.path, if self.value { 1.0 } else { 0.0 })
            .unwrap();
    }
}

impl From<&Node> for BoolControl {
    fn from(node: &Node) -> Self {
        let value = node.widget_type().init_value() > 0.5;
        let path = node.path();
        Self { value, path }
    }
}

#[derive(Debug, Clone)]
pub struct NoteControl2 {
    /// Control for the pitch of the note
    pub note: Control,

    /// Control for the volume of the note
    pub volume: Control,
}

impl From<(&Node, &Node)> for NoteControl2 {
    fn from((note, volume): (&Node, &Node)) -> Self {
        Self {
            note: note.into(),
            volume: volume.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoteControl {
    /// Current value of the control in the DSP
    pub value: f32,

    /// Name for the DSP
    pub path: String,

    /// Raw note, without autotune
    pub raw_value: f32,

    /// Autotune strength
    pub autotune: usize,
}

impl ControlTrait for NoteControl {
    fn send(&mut self, state: &mut StateHandle) {
        state.set_by_path(&self.path, self.value).unwrap();
    }
}

impl NoteControl {
    pub fn set_scaled(
        &mut self,
        value: f32,
        value_range: RangeInclusive<f32>,
        autotune_value: f32,
        autotune_range: RangeInclusive<f32>,
        settings: &Settings,
    ) {
        let range = settings.note_range_f();
        self.raw_value = convert_range(value, value_range, &range);
        self.autotune = convert_range(autotune_value, autotune_range, &(0.0..=5.0)) as usize;
        self.value = smoothstairs(self.raw_value, self.autotune, settings.scale_notes());
    }
}

impl From<&Node> for NoteControl {
    fn from(node: &Node) -> Self {
        let value = node.widget_type().init_value();
        let path = node.path();
        let raw_value = node.widget_type().init_value();
        let autotune = 0;
        Self {
            value,
            path,
            raw_value,
            autotune,
        }
    }
}

trait NodeByPath {
    fn node_by_path(&self, path: &str) -> Option<&Node>;
}

impl NodeByPath for StateHandle {
    fn node_by_path(&self, path: &str) -> Option<&Node> {
        self.params().values().find(|n| n.path() == path)
    }
}

fn convert_range(
    value: f32,
    input_range: RangeInclusive<f32>,
    output_range: &RangeInclusive<f32>,
) -> f32 {
    {
        let in_min = *input_range.start();
        let in_max = *input_range.end();
        let out_min = *output_range.start();
        let out_max = *output_range.end();
        ((((value - in_min) * (out_max - out_min)) / (in_max - in_min)) + out_min)
            .clamp(out_min, out_max)
    }
}

/// Smooth step function loosely "sticking" the value to 0 or 1
/// Assumes that value is between 0 and 1
/// https://en.wikipedia.org/wiki/Smoothstep
fn smoothstep(interval: &RangeInclusive<f32>, x: f32) -> f32 {
    let x = (x - interval.start()) / (interval.end() - interval.start());
    x * x * (3.0 - 2.0 * x)
}

pub fn smoothstairs(value: f32, amount: usize, scale: Vec<MidiNote>) -> f32 {
    let scale: Vec<_> = scale
        .windows(2)
        .map(|w| (w[0].into_byte() as f32)..=(w[1].into_byte() as f32))
        .collect();

    if let Some(interval) = scale.iter().find(|interval| interval.contains(&value)) {
        let mut value = value;

        for _ in 0..amount {
            let smooth = smoothstep(interval, value);
            value = interval.start() + smooth * (interval.end() - interval.start());
        }
        return value;
    }
    value
}
