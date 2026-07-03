//! Power quality indices: EN 50160, IEEE 519-2022, and distribution system indices.
//!
//! ## EN 50160 (European voltage quality standard)
//!
//! Defines supply voltage characteristics for public LV/MV networks.  The
//! standard uses a 1-week observation period with 10-minute statistical
//! intervals for voltage and frequency.
//!
//! ## IEEE 519-2022
//!
//! Defines recommended practices for harmonic control in electrical power
//! systems.  Current harmonic limits depend on the ISC/IL ratio at the PCC.
//!
//! ## PQ Indices
//!
//! Statistical voltage-quality indices aggregated over a measurement period.

use crate::powerquality::waveform::HarmonicComponent;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// EN 50160
// ─────────────────────────────────────────────────────────────────────────────

/// EN 50160 supply voltage quality limits (LV public network).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct En50160Limits {
    /// Maximum total harmonic distortion of voltage [%].
    pub thd_voltage_pct: f64,
    /// Individual voltage harmonic limits: `(order, percent_limit)`.
    ///
    /// EN 50160 Table 1 lists odd harmonics h=3..25 and even harmonics h=2..24.
    pub individual_harmonics: Vec<(usize, f64)>,
    /// Maximum supply voltage variation \[pu\] (default ±0.10).
    pub voltage_variation_pu: f64,
    /// Frequency variation \[Hz\] for interconnected networks (default ±1.0 Hz,
    /// covering 99 % of the week; ±4/−6 Hz covers 100 %).
    pub frequency_variation_hz: f64,
    /// Short-term flicker severity limit (Pst ≤ 1.0 for 95 % of week).
    pub flicker_pst_limit: f64,
    /// Voltage unbalance factor [%] (default 2 %).
    pub unbalance_pct: f64,
}

impl En50160Limits {
    /// Standard EN 50160 limits for LV public networks (230 V / 50 Hz).
    pub fn standard() -> Self {
        // Table 1: individual harmonic voltage limits [%]
        // Odd harmonics (non-multiple of 3): 5, 7, 11, 13, 17, 19, 23, 25
        // Odd harmonics (multiple of 3):     3, 9, 15, 21
        // Even harmonics:                    2, 4, 6..24
        let individual_harmonics = vec![
            (2, 2.0),
            (3, 5.0),
            (4, 1.0),
            (5, 6.0),
            (6, 0.5),
            (7, 5.0),
            (8, 0.5),
            (9, 1.5),
            (10, 0.5),
            (11, 3.5),
            (12, 0.5),
            (13, 3.0),
            (14, 0.5),
            (15, 0.5),
            (17, 2.0),
            (19, 1.5),
            (21, 0.5),
            (23, 1.5),
            (25, 1.5),
        ];

        Self {
            thd_voltage_pct: 8.0,
            individual_harmonics,
            voltage_variation_pu: 0.10,
            frequency_variation_hz: 1.0,
            flicker_pst_limit: 1.0,
            unbalance_pct: 2.0,
        }
    }
}

/// EN 50160 compliance report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct En50160Report {
    /// Whether THD is within the limit.
    pub thd_compliant: bool,
    /// Measured THD [%] (95th percentile of measurement intervals).
    pub thd_measured_pct: f64,
    /// Individual harmonic compliance: `(order, compliant, measured_pct)`.
    pub individual_harmonics_compliant: Vec<(usize, bool, f64)>,
    /// Whether 95 % of 10-min voltage intervals are within ±10 % of nominal.
    pub voltage_variation_compliant: bool,
    /// Whether 99.5 % of 10-s frequency measurements are within the band.
    pub frequency_compliant: bool,
    /// Whether Pst ≤ limit.
    pub flicker_compliant: bool,
    /// Whether voltage unbalance ≤ 2 %.
    pub unbalance_compliant: bool,
    /// Overall compliance (all sub-checks pass).
    pub overall_compliant: bool,
    /// Duration of the observation period \[hours\].
    pub observation_period_hours: f64,
}

/// Check EN 50160 compliance from measurement data.
///
/// # Arguments
/// * `v_rms_timeline`   — 10-minute interval RMS voltages \[pu\] (one per interval)
/// * `frequency_timeline` — 10-second interval frequencies \[Hz\]
/// * `harmonics`        — voltage harmonic components for THD/individual check
/// * `flicker_pst`      — measured short-term flicker severity
/// * `unbalance_pct`    — measured voltage unbalance [%]
/// * `limits`           — [`En50160Limits`] to check against
pub fn check_en50160_compliance(
    v_rms_timeline: &[f64],
    frequency_timeline: &[f64],
    harmonics: &[HarmonicComponent],
    flicker_pst: f64,
    unbalance_pct: f64,
    limits: &En50160Limits,
) -> En50160Report {
    let observation_period_hours = v_rms_timeline.len() as f64 / 6.0; // 10-min intervals

    // ── THD ──────────────────────────────────────────────────────────────────
    let v1 = harmonics
        .first()
        .map(|h| h.magnitude_pu)
        .unwrap_or(1.0)
        .max(1e-15);
    let thd_sq: f64 = harmonics
        .iter()
        .skip(1)
        .map(|h| h.magnitude_pu.powi(2))
        .sum();
    let thd_measured_pct = thd_sq.sqrt() / v1 * 100.0;
    let thd_compliant = thd_measured_pct <= limits.thd_voltage_pct;

    // ── Individual harmonics ──────────────────────────────────────────────────
    let individual_harmonics_compliant: Vec<(usize, bool, f64)> = limits
        .individual_harmonics
        .iter()
        .map(|&(order, limit_pct)| {
            let measured = harmonics
                .iter()
                .find(|h| h.order == order)
                .map(|h| h.magnitude_pu / v1 * 100.0)
                .unwrap_or(0.0);
            (order, measured <= limit_pct, measured)
        })
        .collect();

    // ── Voltage variation ─────────────────────────────────────────────────────
    // 95 % of 10-min intervals must be within ±voltage_variation_pu of 1.0 pu.
    let voltage_variation_compliant = if v_rms_timeline.is_empty() {
        true
    } else {
        let n_within = v_rms_timeline
            .iter()
            .filter(|&&v| (v - 1.0).abs() <= limits.voltage_variation_pu)
            .count();
        (n_within as f64 / v_rms_timeline.len() as f64) >= 0.95
    };

    // ── Frequency ─────────────────────────────────────────────────────────────
    // EN 50160: 99.5 % of 10-s intervals within ±1 Hz (for 50 Hz grid).
    let frequency_compliant = if frequency_timeline.is_empty() {
        true
    } else {
        let nominal_hz = 50.0_f64; // EN 50160 is a European standard
        let n_within = frequency_timeline
            .iter()
            .filter(|&&f| (f - nominal_hz).abs() <= limits.frequency_variation_hz)
            .count();
        (n_within as f64 / frequency_timeline.len() as f64) >= 0.995
    };

    // ── Flicker ───────────────────────────────────────────────────────────────
    let flicker_compliant = flicker_pst <= limits.flicker_pst_limit;

    // ── Voltage unbalance ─────────────────────────────────────────────────────
    let unbalance_compliant = unbalance_pct <= limits.unbalance_pct;

    // ── Overall ───────────────────────────────────────────────────────────────
    let all_ih_compliant = individual_harmonics_compliant.iter().all(|(_, ok, _)| *ok);
    let overall_compliant = thd_compliant
        && all_ih_compliant
        && voltage_variation_compliant
        && frequency_compliant
        && flicker_compliant
        && unbalance_compliant;

    En50160Report {
        thd_compliant,
        thd_measured_pct,
        individual_harmonics_compliant,
        voltage_variation_compliant,
        frequency_compliant,
        flicker_compliant,
        unbalance_compliant,
        overall_compliant,
        observation_period_hours,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// IEEE 519-2022
// ─────────────────────────────────────────────────────────────────────────────

/// IEEE 519-2022 current harmonic limits (Table 2).
///
/// Limits depend on the ratio of short-circuit current (ISC) to maximum
/// demand load current (IL) at the Point of Common Coupling (PCC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ieee519Limits {
    /// ISC / IL ratio used to select these limits.
    pub isc_il_ratio: f64,
    /// Odd harmonic limits: `(max_order, percent_of_IL)`.
    /// Orders above the last entry use the last entry's limit.
    pub odd_harmonics: Vec<(usize, f64)>,
    /// Total demand distortion (TDD) limit [%].
    pub thd_i_pct: f64,
}

impl Ieee519Limits {
    /// Return IEEE 519-2022 Table 2 limits for a given ISC/IL ratio.
    ///
    /// | ISC / IL | h < 11 | 11 ≤ h < 17 | 17 ≤ h < 23 | 23 ≤ h < 35 | 35 ≤ h | TDD |
    /// |----------|--------|------------|------------|------------|-------|-----|
    /// | < 20     |  4.0   |    2.0     |    1.5     |    0.6     |  0.3  | 5.0 |
    /// | 20–50    |  7.0   |    3.5     |    2.5     |    1.0     |  0.5  | 8.0 |
    /// | 50–100   | 10.0   |    4.5     |    4.0     |    1.5     |  0.7  | 12.0|
    /// | 100–1000 | 12.0   |    5.5     |    5.0     |    2.0     |  1.0  | 15.0|
    /// | > 1000   | 15.0   |    7.0     |    6.0     |    2.5     |  1.4  | 20.0|
    pub fn for_isc_ratio(isc_il: f64) -> Self {
        let (odd_harmonics, thd_i_pct) = if isc_il < 20.0 {
            (
                vec![
                    (10, 4.0),
                    (16, 2.0),
                    (22, 1.5),
                    (34, 0.6),
                    (usize::MAX, 0.3),
                ],
                5.0,
            )
        } else if isc_il < 50.0 {
            (
                vec![
                    (10, 7.0),
                    (16, 3.5),
                    (22, 2.5),
                    (34, 1.0),
                    (usize::MAX, 0.5),
                ],
                8.0,
            )
        } else if isc_il < 100.0 {
            (
                vec![
                    (10, 10.0),
                    (16, 4.5),
                    (22, 4.0),
                    (34, 1.5),
                    (usize::MAX, 0.7),
                ],
                12.0,
            )
        } else if isc_il < 1000.0 {
            (
                vec![
                    (10, 12.0),
                    (16, 5.5),
                    (22, 5.0),
                    (34, 2.0),
                    (usize::MAX, 1.0),
                ],
                15.0,
            )
        } else {
            (
                vec![
                    (10, 15.0),
                    (16, 7.0),
                    (22, 6.0),
                    (34, 2.5),
                    (usize::MAX, 1.4),
                ],
                20.0,
            )
        };

        Self {
            isc_il_ratio: isc_il,
            odd_harmonics,
            thd_i_pct,
        }
    }

    /// Lookup the harmonic limit [% of IL] for harmonic order `h`.
    fn limit_for_order(&self, h: usize) -> f64 {
        // Only odd harmonics have explicit limits; even harmonics are typically
        // limited to 25 % of the odd harmonic limit for the same order range.
        if h.is_multiple_of(2) {
            return self.limit_for_order(h + 1) * 0.25;
        }
        for &(max_order, limit) in &self.odd_harmonics {
            if h <= max_order {
                return limit;
            }
        }
        // Fall through to last entry's limit.
        self.odd_harmonics.last().map(|&(_, l)| l).unwrap_or(0.3)
    }
}

/// Check IEEE 519-2022 current harmonic compliance.
///
/// Returns `true` if all individual harmonic magnitudes and TDD are within
/// the limits defined in `limits`.
///
/// # Arguments
/// * `current_harmonics` — harmonic components of the load current [pu of IL]
/// * `il_fundamental`    — fundamental load current magnitude \[pu\] (denominator for TDD)
/// * `limits`            — limits from [`Ieee519Limits::for_isc_ratio`]
pub fn check_ieee519_compliance(
    current_harmonics: &[HarmonicComponent],
    il_fundamental: f64,
    limits: &Ieee519Limits,
) -> bool {
    let il = il_fundamental.max(1e-15);

    // Check TDD: √(Σ_{h≥2} I_h²) / IL × 100
    let tdd_sq: f64 = current_harmonics
        .iter()
        .skip(1)
        .map(|h| h.magnitude_pu.powi(2))
        .sum();
    let tdd_pct = tdd_sq.sqrt() / il * 100.0;
    if tdd_pct > limits.thd_i_pct {
        return false;
    }

    // Check individual harmonics.
    for h in current_harmonics.iter().skip(1) {
        let limit = limits.limit_for_order(h.order);
        let measured_pct = h.magnitude_pu / il * 100.0;
        if measured_pct > limit {
            return false;
        }
    }

    true
}

// ─────────────────────────────────────────────────────────────────────────────
// PQ indices for distribution systems
// ─────────────────────────────────────────────────────────────────────────────

/// Statistical power quality indices for a distribution system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqIndices {
    /// Average per-unit voltage over the observation period.
    pub v_avg_pu: f64,
    /// Minimum per-unit voltage recorded.
    pub v_min_pu: f64,
    /// Maximum per-unit voltage recorded.
    pub v_max_pu: f64,
    /// Standard deviation of per-unit voltage.
    pub v_std_pu: f64,
    /// Percentage of time the voltage is within ±10 % of nominal (|V − 1| < 0.10).
    pub percent_time_within_10pct: f64,
    /// Percentage of time the voltage is within ±5 % of nominal.
    pub percent_time_within_5pct: f64,
    /// Number of intervals where |V − 1| > 0.10.
    pub n_exceedances_10pct: usize,
}

/// Compute statistical PQ indices from a sequence of 10-minute RMS measurements.
///
/// # Arguments
/// * `v_rms_10min`  — per-unit RMS voltage samples (one per 10-min interval)
/// * `nominal_v`    — nominal voltage \[pu\]; typically 1.0
pub fn compute_pq_indices(v_rms_10min: &[f64], nominal_v: f64) -> PqIndices {
    if v_rms_10min.is_empty() {
        return PqIndices {
            v_avg_pu: nominal_v,
            v_min_pu: nominal_v,
            v_max_pu: nominal_v,
            v_std_pu: 0.0,
            percent_time_within_10pct: 100.0,
            percent_time_within_5pct: 100.0,
            n_exceedances_10pct: 0,
        };
    }

    let n = v_rms_10min.len() as f64;
    let v_avg = v_rms_10min.iter().sum::<f64>() / n;
    let v_min = v_rms_10min.iter().copied().fold(f64::INFINITY, f64::min);
    let v_max = v_rms_10min
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);

    let variance = v_rms_10min
        .iter()
        .map(|&v| (v - v_avg).powi(2))
        .sum::<f64>()
        / n;
    let v_std = variance.sqrt();

    let n_within_10 = v_rms_10min
        .iter()
        .filter(|&&v| (v - nominal_v).abs() <= 0.10)
        .count();
    let n_within_5 = v_rms_10min
        .iter()
        .filter(|&&v| (v - nominal_v).abs() <= 0.05)
        .count();
    let n_exc_10 = v_rms_10min.len() - n_within_10;

    PqIndices {
        v_avg_pu: v_avg,
        v_min_pu: v_min,
        v_max_pu: v_max,
        v_std_pu: v_std,
        percent_time_within_10pct: n_within_10 as f64 / v_rms_10min.len() as f64 * 100.0,
        percent_time_within_5pct: n_within_5 as f64 / v_rms_10min.len() as f64 * 100.0,
        n_exceedances_10pct: n_exc_10,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::powerquality::waveform::HarmonicComponent;

    fn make_harmonic(order: usize, mag: f64) -> HarmonicComponent {
        HarmonicComponent {
            order,
            magnitude_pu: mag,
            phase_rad: 0.0,
            power: 0.0,
        }
    }

    #[test]
    fn test_en50160_compliant_network() {
        // Ideal network: V = 1.0 pu, f = 50 Hz, pure fundamental.
        let v_rms = vec![1.0_f64; 1008]; // 1 week of 10-min intervals
        let freq = vec![50.0_f64; 6048]; // 10-s intervals
        let harmonics = vec![make_harmonic(1, 1.0)]; // pure fundamental → THD = 0
        let limits = En50160Limits::standard();
        let report = check_en50160_compliance(&v_rms, &freq, &harmonics, 0.3, 0.5, &limits);
        assert!(
            report.overall_compliant,
            "Ideal network should be EN 50160 compliant"
        );
        assert!(report.thd_compliant);
        assert!(report.voltage_variation_compliant);
        assert!(report.frequency_compliant);
    }

    #[test]
    fn test_en50160_thd_violation() {
        let v_rms = vec![1.0_f64; 100];
        let freq = vec![50.0_f64; 100];
        // 10 % 3rd harmonic → THD = 10 % > 8 % limit
        let harmonics = vec![make_harmonic(1, 1.0), make_harmonic(3, 0.10)];
        let limits = En50160Limits::standard();
        let report = check_en50160_compliance(&v_rms, &freq, &harmonics, 0.3, 0.5, &limits);
        assert!(!report.thd_compliant, "THD = 10% should fail the 8% limit");
        assert!(!report.overall_compliant);
    }

    #[test]
    fn test_en50160_voltage_variation_violation() {
        // 10 % of intervals outside ±10 % → fails 95 % criterion.
        let mut v_rms = vec![1.0_f64; 900];
        v_rms.extend(vec![1.15_f64; 100]); // 10 % are at 115 %
        let freq = vec![50.0_f64; 100];
        let harmonics = vec![make_harmonic(1, 1.0)];
        let limits = En50160Limits::standard();
        let report = check_en50160_compliance(&v_rms, &freq, &harmonics, 0.3, 0.5, &limits);
        assert!(!report.voltage_variation_compliant);
    }

    #[test]
    fn test_ieee519_compliant_low_harmonics() {
        // Fundamental only → TDD = 0 → always compliant.
        let harmonics = vec![make_harmonic(1, 1.0)];
        let limits = Ieee519Limits::for_isc_ratio(50.0);
        assert!(check_ieee519_compliance(&harmonics, 1.0, &limits));
    }

    #[test]
    fn test_ieee519_violation_high_harmonics() {
        // ISC/IL < 20: h < 11 limit = 4 % of IL.
        // 5th harmonic at 6 % → violation.
        let harmonics = vec![
            make_harmonic(1, 1.0),
            make_harmonic(5, 0.06), // 6 % of IL=1.0 → > 4 % limit
        ];
        let limits = Ieee519Limits::for_isc_ratio(10.0);
        assert!(
            !check_ieee519_compliance(&harmonics, 1.0, &limits),
            "6% 5th harmonic should violate IEEE 519 for ISC/IL < 20"
        );
    }

    #[test]
    fn test_pq_indices_range() {
        let v: Vec<f64> = (0..100).map(|i| 0.98 + 0.04 * (i as f64 / 100.0)).collect();
        let idx = compute_pq_indices(&v, 1.0);
        assert!(idx.v_min_pu <= idx.v_avg_pu);
        assert!(idx.v_avg_pu <= idx.v_max_pu);
        assert!(idx.v_avg_pu > 0.0 && idx.v_avg_pu < 2.0);
        assert!(idx.percent_time_within_10pct >= 0.0 && idx.percent_time_within_10pct <= 100.0);
        assert!(idx.percent_time_within_5pct >= 0.0 && idx.percent_time_within_5pct <= 100.0);
    }

    #[test]
    fn test_pq_indices_empty() {
        let idx = compute_pq_indices(&[], 1.0);
        assert_eq!(idx.v_avg_pu, 1.0);
        assert_eq!(idx.n_exceedances_10pct, 0);
    }

    #[test]
    fn test_pq_indices_all_nominal() {
        let v = vec![1.0_f64; 200];
        let idx = compute_pq_indices(&v, 1.0);
        assert_eq!(idx.percent_time_within_10pct, 100.0);
        assert_eq!(idx.percent_time_within_5pct, 100.0);
        assert_eq!(idx.n_exceedances_10pct, 0);
        assert!((idx.v_std_pu).abs() < 1e-12);
    }

    #[test]
    fn test_ieee519_limits_isc_ratio_boundaries() {
        for &ratio in &[5.0, 30.0, 75.0, 500.0, 2000.0] {
            let limits = Ieee519Limits::for_isc_ratio(ratio);
            assert!(limits.thd_i_pct > 0.0);
            assert!(!limits.odd_harmonics.is_empty());
        }
    }

    #[test]
    fn test_en50160_flicker_violation() {
        let v_rms = vec![1.0_f64; 100];
        let freq = vec![50.0_f64; 100];
        let harmonics = vec![make_harmonic(1, 1.0)];
        let limits = En50160Limits::standard();
        // Pst = 1.5 > limit of 1.0
        let report = check_en50160_compliance(&v_rms, &freq, &harmonics, 1.5, 0.5, &limits);
        assert!(!report.flicker_compliant);
        assert!(!report.overall_compliant);
    }

    #[test]
    fn test_thd_zero_from_pure_fundamental_harmonics() {
        let v_rms = vec![1.0_f64; 100];
        let freq = vec![50.0_f64; 100];
        let harmonics = vec![make_harmonic(1, 1.0)];
        let limits = En50160Limits::standard();
        let report = check_en50160_compliance(&v_rms, &freq, &harmonics, 0.3, 0.5, &limits);
        assert!(
            report.thd_measured_pct < 1e-9,
            "pure fundamental should give THD = 0%"
        );
    }

    #[test]
    fn test_thd_known_value_5pct_5th_harmonic() {
        let v_rms = vec![1.0_f64; 100];
        let freq = vec![50.0_f64; 100];
        let harmonics = vec![make_harmonic(1, 1.0), make_harmonic(5, 0.05)];
        let limits = En50160Limits::standard();
        let report = check_en50160_compliance(&v_rms, &freq, &harmonics, 0.3, 0.5, &limits);
        // THD = sqrt(0.05^2) / 1.0 * 100 = 5.0%
        assert!(
            (report.thd_measured_pct - 5.0).abs() < 0.001,
            "THD should be 5.0%, got {}",
            report.thd_measured_pct
        );
    }

    #[test]
    fn test_k_factor_pure_fundamental() {
        let harmonics = vec![HarmonicComponent {
            order: 1,
            magnitude_pu: 1.0,
            phase_rad: 0.0,
            power: 0.0,
        }];
        let k = crate::powerquality::waveform::compute_k_factor(&harmonics);
        assert!(
            (k - 1.0).abs() < 1e-9,
            "K-factor for pure fundamental should be 1.0, got {k}"
        );
    }

    #[test]
    fn test_crest_factor_pure_sine_via_waveform() {
        let sample_rate = 10_000.0_f64;
        let freq = 50.0_f64;
        let n_samples = (5.0 * sample_rate / freq) as usize; // 1000 samples = 5 cycles
        let wave: Vec<f64> = (0..n_samples)
            .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / sample_rate).sin())
            .collect();
        let metrics =
            crate::powerquality::waveform::analyze_waveform(&wave, &wave, sample_rate, freq, 10)
                .expect("analyze_waveform failed");
        assert!(
            (metrics.crest_factor - std::f64::consts::SQRT_2).abs() < 0.05,
            "Crest factor for pure sine should be sqrt(2) ≈ {}, got {}",
            std::f64::consts::SQRT_2,
            metrics.crest_factor
        );
    }

    #[test]
    fn test_ieee519_tdd_violation() {
        // ISC/IL < 20: TDD limit = 5%.
        // h=5 at 0.04 pu, h=7 at 0.04 pu: TDD = sqrt(0.04^2 + 0.04^2)*100 ≈ 5.66% > 5% limit
        let harmonics = vec![
            make_harmonic(1, 1.0),
            make_harmonic(5, 0.04),
            make_harmonic(7, 0.04),
        ];
        let limits = Ieee519Limits::for_isc_ratio(10.0); // ISC/IL < 20 → TDD limit = 5%
        assert!(
            !check_ieee519_compliance(&harmonics, 1.0, &limits),
            "TDD ≈ 5.66% should violate the 5% limit"
        );
    }
}
