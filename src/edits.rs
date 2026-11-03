//! Per-step manual edits: hit overrides, accent, probability, ratchets and microtiming.
//!
//! The audio thread must never take a lock, so the persistent `EditStore` mirrors its data in
//! atomics. `EditView` is the plain, copyable snapshot the generators and scheduler read. A
//! revision counter makes a manual edit look like a parameter change to the existing snapshot
//! comparison.

use std::fmt;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering::Relaxed};
use std::sync::Arc;

use nice_plug::params::persist::PersistentField;
use serde::{Deserialize, Serialize};

use crate::sequence::{MAX_STEPS, TRACKS};

/// Hit override: 0 = follow the generator, 1 = force on, 2 = force off.
pub const HIT_AUTO: u8 = 0;
pub const HIT_ON: u8 = 1;
pub const HIT_OFF: u8 = 2;
/// Probability override: 255 = follow the global probability.
pub const PROB_AUTO: u8 = 255;
/// Accent override: 0 = follow the generated accent.
pub const ACCENT_AUTO: u8 = 0;
/// Ratchet count; 1 means no ratchet.
pub const RATCHET_AUTO: u8 = 1;

/// Plain snapshot consumed by generation, the scheduler, export and preview.
#[derive(Clone, Debug, PartialEq)]
pub struct EditView {
    pub hit: [[u8; MAX_STEPS]; TRACKS],
    pub accent: [[u8; MAX_STEPS]; TRACKS],
    pub probability: [[u8; MAX_STEPS]; TRACKS],
    pub ratchet: [[u8; MAX_STEPS]; TRACKS],
    /// Microtiming as a fraction of one step, -127..127 (0 = on the grid).
    pub offset: [[i8; MAX_STEPS]; TRACKS],
}

impl Default for EditView {
    fn default() -> Self {
        Self {
            hit: [[HIT_AUTO; MAX_STEPS]; TRACKS],
            accent: [[ACCENT_AUTO; MAX_STEPS]; TRACKS],
            probability: [[PROB_AUTO; MAX_STEPS]; TRACKS],
            ratchet: [[RATCHET_AUTO; MAX_STEPS]; TRACKS],
            offset: [[0; MAX_STEPS]; TRACKS],
        }
    }
}

impl EditView {
    pub fn hit_override(&self, lane: usize, step: usize) -> u8 {
        self.hit[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)]
    }

    /// `None` means the global probability still applies.
    pub fn probability_at(&self, lane: usize, step: usize) -> Option<f32> {
        let value = self.probability[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)];
        (value != PROB_AUTO).then(|| value as f32 / 254.0)
    }

    /// `None` means the generated accent still applies.
    pub fn accent_at(&self, lane: usize, step: usize) -> Option<f32> {
        let value = self.accent[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)];
        (value != ACCENT_AUTO).then(|| (value as f32 / 255.0).clamp(0.05, 1.0))
    }

    pub fn ratchet_at(&self, lane: usize, step: usize) -> u8 {
        self.ratchet[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)].clamp(1, 4)
    }

    pub fn offset_at(&self, lane: usize, step: usize) -> f32 {
        self.offset[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)] as f32 / 127.0
    }
}

/// One edited cell in the project state. Only non-default cells are serialized.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditCell {
    pub lane: u8,
    pub step: u16,
    #[serde(default)]
    pub hit: u8,
    #[serde(default)]
    pub accent: u8,
    #[serde(default = "auto_probability")]
    pub probability: u8,
    #[serde(default = "auto_ratchet")]
    pub ratchet: u8,
    #[serde(default)]
    pub offset: i8,
}

fn auto_probability() -> u8 { PROB_AUTO }
fn auto_ratchet() -> u8 { RATCHET_AUTO }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EditState {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub cells: Vec<EditCell>,
}

/// Lock-free mirror of the edit state. Reads and writes are single-byte atomics; the revision is
/// bumped after a change so the audio thread knows to refresh its plain `EditView`.
pub struct EditStore {
    hit: [[AtomicU8; MAX_STEPS]; TRACKS],
    accent: [[AtomicU8; MAX_STEPS]; TRACKS],
    probability: [[AtomicU8; MAX_STEPS]; TRACKS],
    ratchet: [[AtomicU8; MAX_STEPS]; TRACKS],
    offset: [[AtomicU8; MAX_STEPS]; TRACKS],
    revision: AtomicU32,
}

impl Default for EditStore {
    fn default() -> Self {
        Self {
            hit: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU8::new(HIT_AUTO))),
            accent: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU8::new(ACCENT_AUTO))),
            probability: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU8::new(PROB_AUTO))),
            ratchet: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU8::new(RATCHET_AUTO))),
            offset: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU8::new(0))),
            revision: AtomicU32::new(0),
        }
    }
}

impl fmt::Debug for EditStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EditStore").field("revision", &self.revision()).finish()
    }
}

impl EditStore {
    pub fn revision(&self) -> u32 { self.revision.load(Relaxed) }

    fn bump(&self) { self.revision.fetch_add(1, Relaxed); }

    pub fn hit(&self, lane: usize, step: usize) -> u8 {
        self.hit[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)].load(Relaxed)
    }

    pub fn set_hit(&self, lane: usize, step: usize, value: u8) {
        self.hit[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)].store(value.min(HIT_OFF), Relaxed);
        self.bump();
    }

    pub fn accent(&self, lane: usize, step: usize) -> u8 {
        self.accent[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)].load(Relaxed)
    }

    pub fn set_accent(&self, lane: usize, step: usize, value: u8) {
        self.accent[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)].store(value, Relaxed);
        self.bump();
    }

    pub fn probability(&self, lane: usize, step: usize) -> u8 {
        self.probability[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)].load(Relaxed)
    }

    pub fn set_probability(&self, lane: usize, step: usize, value: u8) {
        self.probability[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)].store(value, Relaxed);
        self.bump();
    }

    pub fn ratchet(&self, lane: usize, step: usize) -> u8 {
        self.ratchet[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)].load(Relaxed).clamp(1, 4)
    }

    pub fn set_ratchet(&self, lane: usize, step: usize, value: u8) {
        self.ratchet[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)]
            .store(value.clamp(1, 4), Relaxed);
        self.bump();
    }

    pub fn offset(&self, lane: usize, step: usize) -> i8 {
        self.offset[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)].load(Relaxed) as i8
    }

    pub fn set_offset(&self, lane: usize, step: usize, value: i8) {
        self.offset[lane.min(TRACKS - 1)][step.min(MAX_STEPS - 1)].store(value as u8, Relaxed);
        self.bump();
    }

    pub fn clear_cell(&self, lane: usize, step: usize) {
        self.set_hit(lane, step, HIT_AUTO);
        self.set_accent(lane, step, ACCENT_AUTO);
        self.set_probability(lane, step, PROB_AUTO);
        self.set_ratchet(lane, step, RATCHET_AUTO);
        self.set_offset(lane, step, 0);
    }

    pub fn clear_all(&self) {
        for lane in 0..TRACKS {
            for step in 0..MAX_STEPS {
                self.hit[lane][step].store(HIT_AUTO, Relaxed);
                self.accent[lane][step].store(ACCENT_AUTO, Relaxed);
                self.probability[lane][step].store(PROB_AUTO, Relaxed);
                self.ratchet[lane][step].store(RATCHET_AUTO, Relaxed);
                self.offset[lane][step].store(0, Relaxed);
            }
        }
        self.bump();
    }

    /// Fill the audio thread's plain snapshot. Called only when the revision changes.
    pub fn fill(&self, out: &mut EditView) {
        for lane in 0..TRACKS {
            for step in 0..MAX_STEPS {
                out.hit[lane][step] = self.hit[lane][step].load(Relaxed);
                out.accent[lane][step] = self.accent[lane][step].load(Relaxed);
                out.probability[lane][step] = self.probability[lane][step].load(Relaxed);
                out.ratchet[lane][step] = self.ratchet[lane][step].load(Relaxed);
                out.offset[lane][step] = self.offset[lane][step].load(Relaxed) as i8;
            }
        }
    }

    pub fn snapshot(&self) -> EditState {
        let mut cells = Vec::new();
        for lane in 0..TRACKS {
            for step in 0..MAX_STEPS {
                let hit = self.hit[lane][step].load(Relaxed);
                let accent = self.accent[lane][step].load(Relaxed);
                let probability = self.probability[lane][step].load(Relaxed);
                let ratchet = self.ratchet[lane][step].load(Relaxed);
                let offset = self.offset[lane][step].load(Relaxed) as i8;
                if hit != HIT_AUTO
                    || accent != ACCENT_AUTO
                    || probability != PROB_AUTO
                    || ratchet != RATCHET_AUTO
                    || offset != 0
                {
                    cells.push(EditCell { lane: lane as u8, step: step as u16, hit, accent, probability, ratchet, offset });
                }
            }
        }
        EditState { version: 1, cells }
    }

    pub fn apply(&self, state: &EditState) {
        self.clear_all();
        for cell in &state.cells {
            let lane = cell.lane as usize;
            let step = cell.step as usize;
            if lane >= TRACKS || step >= MAX_STEPS { continue; }
            self.hit[lane][step].store(cell.hit.min(HIT_OFF), Relaxed);
            self.accent[lane][step].store(cell.accent, Relaxed);
            self.probability[lane][step].store(cell.probability, Relaxed);
            self.ratchet[lane][step].store(cell.ratchet.clamp(1, 4), Relaxed);
            self.offset[lane][step].store(cell.offset as u8, Relaxed);
        }
        self.bump();
    }
}

impl<'a> PersistentField<'a, EditState> for Arc<EditStore> {
    fn set(&self, new_value: EditState) { self.apply(&new_value); }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&EditState) -> R,
    {
        f(&self.snapshot())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_view_leaves_every_cell_to_the_generator() {
        let view = EditView::default();
        for lane in 0..TRACKS {
            for step in 0..MAX_STEPS {
                assert_eq!(view.hit_override(lane, step), HIT_AUTO);
                assert_eq!(view.probability_at(lane, step), None);
                assert_eq!(view.accent_at(lane, step), None);
                assert_eq!(view.ratchet_at(lane, step), 1);
                assert_eq!(view.offset_at(lane, step), 0.0);
            }
        }
    }

    #[test]
    fn edits_survive_a_snapshot_round_trip() {
        let store = EditStore::default();
        store.set_hit(2, 7, HIT_OFF);
        store.set_probability(2, 7, 127);
        store.set_accent(7, 1015, 200);
        store.set_ratchet(0, 0, 4);
        store.set_offset(1, 3, -64);
        let state = store.snapshot();
        let restored = EditStore::default();
        restored.apply(&state);
        let mut original = EditView::default();
        let mut copy = EditView::default();
        store.fill(&mut original);
        restored.fill(&mut copy);
        assert_eq!(original, copy);
    }

    #[test]
    fn clearing_a_cell_restores_the_generator_defaults() {
        let store = EditStore::default();
        store.set_hit(1, 1, HIT_ON);
        store.set_probability(1, 1, 10);
        store.clear_cell(1, 1);
        let mut view = EditView::default();
        store.fill(&mut view);
        assert_eq!(view.hit_override(1, 1), HIT_AUTO);
        assert_eq!(view.probability_at(1, 1), None);
    }

    #[test]
    fn every_edit_bumps_the_revision() {
        let store = EditStore::default();
        let start = store.revision();
        store.set_hit(0, 0, HIT_ON);
        store.set_accent(0, 0, 1);
        store.set_probability(0, 0, 1);
        store.set_ratchet(0, 0, 2);
        store.set_offset(0, 0, 1);
        store.clear_cell(0, 0);
        assert!(store.revision() > start + 5);
    }
}
