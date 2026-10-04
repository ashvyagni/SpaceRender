//! Sound, synthesised on the fly (no recordings, nothing to license).
//!
//! * An ambient drone: detuned low pads that swell and fade on slow, unrelated cycles over
//!   a filtered "solar wind" hiss — never quite repeating.
//! * Impacts: a deep thump and a rumble of brown noise, louder for bigger blasts.
//! * Supernovae: a long, rising roar.
//! * Milestones: a soft bell (at most one every few seconds).
//!
//! Space is silent, of course: this is a soundtrack, not a simulation.

use bevy::audio::{AddAudioSource, AudioPlayer, AudioSink, AudioSinkPlayback, Decodable, PlaybackSettings, Source, Volume};
use bevy::prelude::*;
use std::time::Duration;

use crate::persistence::UserSettings;
use crate::sim::Sim;
use crate::state::AppState;

const SR: u32 = 44_100;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SynthKind {
    Ambient,
    Rumble,
    Roar,
    Bell,
}

#[derive(Asset, TypePath, Clone, Copy, Debug)]
pub struct Synth {
    pub kind: SynthKind,
    /// 0..1: size of the event.
    pub intensity: f32,
    pub seed: u32,
}

pub struct SynthDecoder {
    s: Synth,
    n: u64,
    rng: u32,
    brown: f32,
    lp: f32,
    lp2: f32,
}

impl SynthDecoder {
    fn white(&mut self) -> f32 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn duration_s(&self) -> Option<f32> {
        match self.s.kind {
            SynthKind::Ambient => None,
            SynthKind::Rumble => Some(5.5),
            SynthKind::Roar => Some(10.0),
            SynthKind::Bell => Some(3.0),
        }
    }
}

const TAU: f32 = std::f32::consts::TAU;

impl Iterator for SynthDecoder {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let t = self.n as f32 / SR as f32;
        if self.duration_s().is_some_and(|d| t >= d) {
            return None;
        }
        self.n += 1;
        let w = self.white();
        self.brown = (self.brown + 0.02 * w) / 1.02;
        let i = self.s.intensity.clamp(0.0, 1.0);
        let v = match self.s.kind {
            SynthKind::Ambient => {
                // Five voices of a suspended chord, each swelling on its own slow cycle.
                let voices = [(55.0, 23.0), (82.41, 31.0), (110.0, 41.0), (164.81, 53.0), (220.4, 67.0)];
                let mut pad = 0.0;
                for (k, (f, period)) in voices.iter().enumerate() {
                    let swell = 0.5 + 0.5 * (TAU * t / period + k as f32 * 1.7).sin();
                    let detune = 1.0 + 0.0015 * (TAU * t / 11.0 + k as f32).sin();
                    pad += swell * swell * ((TAU * f * detune * t).sin() + 0.3 * (TAU * f * 2.003 * t).sin());
                }
                // Wind: white noise through a slowly sweeping one-pole low-pass.
                let cut = 0.004 + 0.012 * (0.5 + 0.5 * (TAU * t / 37.0).sin());
                self.lp += cut * (w - self.lp);
                let wind = self.lp * (0.6 + 0.4 * (TAU * t / 19.0).sin());
                0.035 * pad + 0.9 * wind
            }
            SynthKind::Rumble => {
                let env = (-t * 0.9).exp() * (1.0 - (-t * 60.0).exp());
                let thump = (TAU * (42.0 - 14.0 * t.min(1.0)) * t).sin() * (-t * 3.0).exp();
                self.lp += 0.03 * (self.brown - self.lp);
                env * (0.7 * thump + 5.0 * self.lp) * (0.35 + 0.65 * i)
            }
            SynthKind::Roar => {
                let attack = (t / 1.8).min(1.0);
                let env = attack * attack * (-(t - 1.8).max(0.0) * 0.38).exp();
                self.lp += 0.05 * (self.brown - self.lp);
                self.lp2 += 0.2 * (w - self.lp2);
                let growl = (TAU * 31.0 * t + 3.0 * (TAU * 0.7 * t).sin()).sin();
                env * (6.0 * self.lp + 0.12 * self.lp2 + 0.35 * growl)
            }
            SynthKind::Bell => {
                // Inharmonic partials (a small bell), each decaying at its own rate.
                let f0 = [523.25, 587.33, 659.25, 783.99][(self.s.seed % 4) as usize];
                let mut b = 0.0;
                for (ratio, decay, amp) in [(1.0, 1.4, 1.0), (2.76, 2.6, 0.45), (5.40, 4.0, 0.25), (8.93, 6.0, 0.12)] {
                    b += amp * (TAU * f0 * ratio * t).sin() * (-t * decay).exp();
                }
                0.12 * b * (1.0 - (-t * 200.0).exp())
            }
        };
        Some((v * 1.2).tanh() * 0.8)
    }
}

impl Source for SynthDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        SR
    }
    fn total_duration(&self) -> Option<Duration> {
        self.duration_s().map(Duration::from_secs_f32)
    }
}

impl Decodable for Synth {
    type DecoderItem = f32;
    type Decoder = SynthDecoder;
    fn decoder(&self) -> SynthDecoder {
        SynthDecoder { s: *self, n: 0, rng: 0x9E37_79B9 ^ self.seed.wrapping_mul(2_654_435_761), brown: 0.0, lp: 0.0, lp2: 0.0 }
    }
}

/// A one-shot sound requested by another system.
#[derive(Message, Clone, Copy)]
pub struct SoundCue {
    pub kind: SynthKind,
    pub intensity: f32,
}

#[derive(Component)]
struct AmbientSound;

#[derive(Resource, Default)]
struct SoundWatch {
    events: usize,
    blasts: usize,
    last_bell: f64,
    generation: Option<u64>,
}

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<Synth>()
            .add_message::<SoundCue>()
            .init_resource::<SoundWatch>()
            .add_systems(OnEnter(AppState::Observing), start_ambient)
            .add_systems(OnExit(AppState::Observing), stop_ambient)
            .add_systems(Update, (watch_events, play_cues, ambient_volume).chain().run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)));
    }
}

fn start_ambient(mut commands: Commands, mut synths: ResMut<Assets<Synth>>, settings: Res<UserSettings>, args: Res<crate::args::Args>) {
    if args.capture.is_some() {
        return;
    }
    let h = synths.add(Synth { kind: SynthKind::Ambient, intensity: 1.0, seed: 1 });
    commands.spawn((AudioPlayer(h), PlaybackSettings::LOOP.with_volume(Volume::Linear(settings.sound_volume * 0.5)), AmbientSound));
}

fn stop_ambient(mut commands: Commands, q: Query<Entity, With<AmbientSound>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn ambient_volume(settings: Res<UserSettings>, mut q: Query<&mut AudioSink, With<AmbientSound>>) {
    if !settings.is_changed() {
        return;
    }
    for mut s in &mut q {
        s.set_volume(Volume::Linear(settings.sound_volume * 0.5));
    }
}

/// Supernovae and milestones, read from the simulation.
fn watch_events(sim: Res<Sim>, mut w: ResMut<SoundWatch>, mut cues: MessageWriter<SoundCue>, time: Res<Time>) {
    let u = &sim.universe;
    // Fresh universe, rewind or undo: start counting from here, silently.
    if w.generation != Some(sim.generation) {
        w.generation = Some(sim.generation);
        w.events = u.history.events.len();
        w.blasts = u.blasts.len();
        return;
    }
    if u.blasts.len() > w.blasts {
        cues.write(SoundCue { kind: SynthKind::Roar, intensity: 1.0 });
    }
    w.blasts = u.blasts.len();
    let now = time.elapsed_secs_f64();
    let n = u.history.events.len();
    if n > w.events && now - w.last_bell > 4.0 && u.history.events[w.events.min(n)..].iter().any(|e| e.importance >= 5) {
        w.last_bell = now;
        cues.write(SoundCue { kind: SynthKind::Bell, intensity: 0.5 });
    }
    w.events = n;
}

fn play_cues(mut commands: Commands, mut cues: MessageReader<SoundCue>, mut synths: ResMut<Assets<Synth>>, settings: Res<UserSettings>, args: Res<crate::args::Args>, mut seed: Local<u32>) {
    if args.capture.is_some() || settings.sound_volume <= 0.0 {
        cues.clear();
        return;
    }
    for c in cues.read() {
        *seed = seed.wrapping_add(1);
        let h = synths.add(Synth { kind: c.kind, intensity: c.intensity, seed: *seed });
        let gain = match c.kind {
            SynthKind::Bell => 0.5,
            _ => 1.0,
        };
        commands.spawn((AudioPlayer(h), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(settings.sound_volume * gain))));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(kind: SynthKind) -> Vec<f32> {
        Synth { kind, intensity: 1.0, seed: 3 }.decoder().take(SR as usize * 12).collect()
    }

    #[test]
    fn one_shots_end_and_nothing_clips() {
        for k in [SynthKind::Rumble, SynthKind::Roar, SynthKind::Bell] {
            let s = render(k);
            assert!(s.len() < SR as usize * 11, "{k:?} must end");
            let peak = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.05 && peak <= 0.8, "{k:?} peak {peak}");
        }
        let amb = render(SynthKind::Ambient);
        assert_eq!(amb.len(), SR as usize * 12, "ambient never ends");
        assert!(amb.iter().all(|x| x.is_finite() && x.abs() <= 0.8));
    }
}
