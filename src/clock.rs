//! Timing math derived from the sequencer mode.
//!
//! The host gives us a position in quarter notes. The plug-in never asks the
//! user for a meter: each mode declares its own step unit, and the generated
//! pattern (length and per-lane periods) implies the meter. Everything here is
//! rational so unusual divisions stay exact.

/// Small reduced fraction. `den` is always non-zero and positive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frac {
    pub num: u32,
    pub den: u32,
}

impl Frac {
    pub const fn new(num: u32, den: u32) -> Self {
        Self { num, den }
    }

    pub fn reduced(num: u32, den: u32) -> Self {
        let den = den.max(1);
        let g = gcd_u32(num.max(1), den);
        Self { num: num.max(1) / g, den: den / g }
    }

    pub fn as_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    pub fn inverse(self) -> Self {
        Self::reduced(self.den, self.num)
    }

    pub fn is_zero(self) -> bool {
        self.num == 0
    }
}

impl Default for Frac {
    fn default() -> Self {
        Self { num: 1, den: 1 }
    }
}

/// A step subdivision owned by a mode. `steps_per_quarter` is exact: 1/16 is
/// 4, 1/8T is 3, 1/16T is 6. `note_den` is the note value used by a MIDI time
/// signature when the unit is a plain power-of-two note (1/16 -> 16); triplets
/// and dotted units have no exact signature denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepUnit {
    Quarter,
    Eighth,
    EighthTriplet,
    Sixteenth,
    SixteenthTriplet,
    ThirtySecond,
}

impl StepUnit {
    pub fn steps_per_quarter(self) -> Frac {
        match self {
            Self::Quarter => Frac::new(1, 1),
            Self::Eighth => Frac::new(2, 1),
            Self::EighthTriplet => Frac::new(3, 1),
            Self::Sixteenth => Frac::new(4, 1),
            Self::SixteenthTriplet => Frac::new(6, 1),
            Self::ThirtySecond => Frac::new(8, 1),
        }
    }

    pub fn note_den(self) -> Option<u32> {
        match self {
            Self::Quarter => Some(4),
            Self::Eighth => Some(8),
            Self::Sixteenth => Some(16),
            Self::ThirtySecond => Some(32),
            Self::EighthTriplet | Self::SixteenthTriplet => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Quarter => "1/4",
            Self::Eighth => "1/8",
            Self::EighthTriplet => "1/8T",
            Self::Sixteenth => "1/16",
            Self::SixteenthTriplet => "1/16T",
            Self::ThirtySecond => "1/32",
        }
    }
}

/// A bar grouping implied by a pattern. There is no host or user meter here;
/// values are produced by [`meter_from_steps`] from the generated period.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meter {
    pub num: u32,
    pub den: u32,
}

impl Meter {
    pub const fn new(num: u32, den: u32) -> Self {
        Self { num, den }
    }

    /// Length of one bar in quarter notes.
    pub fn bar_beats(self) -> f64 {
        self.num as f64 * 4.0 / self.den.max(1) as f64
    }

}

impl Default for Meter {
    fn default() -> Self {
        Self { num: 4, den: 4 }
    }
}

/// Readable name for a pattern: `4/4`, `13/16`, `12/8`, ...
pub fn meter_label(meter: Option<Meter>, unit: StepUnit, loop_steps: u32) -> String {
    match meter {
        Some(m) => format!("{}/{} · step {}", m.num, m.den, unit.label()),
        None => format!("polymetric · step {} · loop {}", unit.label(), loop_steps),
    }
}

/// Implied meter for a repeating pattern of `steps` cells at `unit`.
///
/// The bar is the pattern period. We halve long even numerators until the
/// signature is compact (16/16 -> 8/8 -> 4/4) but keep musically useful odd
/// bars such as 13/16, 12/8 and 15/8. Multi-bar or unsupported units return
/// `None`: the caller shows the loop length instead of inventing a signature.
pub fn meter_from_steps(steps: u32, unit: StepUnit) -> Option<Meter> {
    let den = unit.note_den()?;
    if steps == 0 {
        return None;
    }
    let mut n = steps;
    let mut d = den;
    while n > 16 && d > 4 && n % 2 == 0 {
        n /= 2;
        d /= 2;
    }
    if n == d && d > 4 {
        n = 4;
        d = 4;
    }
    if (2..=16).contains(&n) {
        Some(Meter::new(n, d))
    } else {
        None
    }
}

/// Lanes realign after the least common multiple of their integer periods.
/// Returns the exact value and whether it exceeds `cap`. Uses gcd reduction so
/// intermediate products cannot overflow before the cap check.
pub fn lcm_capped(lane_steps: &[u16], cap: u64) -> (u64, bool) {
    let mut out = 1u64;
    for &n in lane_steps {
        let n = n.max(1) as u64;
        let g = gcd_u64(out, n);
        out = (out / g).saturating_mul(n);
        if out > cap {
            return (out, true);
        }
    }
    (out, false)
}

pub fn gcd_u32(a: u32, b: u32) -> u32 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a.max(1)
}

pub fn gcd_u64(a: u64, b: u64) -> u64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implied_meter_uses_the_mode_unit_and_loop() {
        assert_eq!(meter_from_steps(16, StepUnit::Sixteenth), Some(Meter::new(4, 4)));
        assert_eq!(meter_from_steps(13, StepUnit::Sixteenth), Some(Meter::new(13, 16)));
        assert_eq!(meter_from_steps(12, StepUnit::Eighth), Some(Meter::new(12, 8)));
        assert_eq!(meter_from_steps(15, StepUnit::Eighth), Some(Meter::new(15, 8)));
        assert_eq!(meter_from_steps(6, StepUnit::Eighth), Some(Meter::new(6, 8)));
        // 81/16 does not compress to a usable numerator; the UI shows the loop instead.
        assert_eq!(meter_from_steps(81, StepUnit::Sixteenth), None);
        // Triplet units have no power-of-two signature denominator.
        assert_eq!(meter_from_steps(16, StepUnit::SixteenthTriplet), None);
    }

    #[test]
    fn lcm_reports_exact_and_capped() {
        assert_eq!(lcm_capped(&[13, 7], 4096), (91, false));
        assert_eq!(lcm_capped(&[2, 3, 4, 5, 6, 7, 8], 4096), (840, false));
        let (value, capped) = lcm_capped(&[1021, 1022, 1023], 4096);
        assert!(value > 4096);
        assert!(capped);
    }

    #[test]
    fn fractions_reduce() {
        assert_eq!(Frac::reduced(14, 8), Frac { num: 7, den: 4 });
        assert_eq!(Frac::reduced(7, 4).inverse(), Frac { num: 4, den: 7 });
    }
}
