//! Power quality standards compliance checking.
//!
//! Provides structured compliance checkers for the major international power
//! quality standards used in grid monitoring, equipment certification, and
//! transmission planning:
//!
//! | Checker | Standard | Scope |
//! |---------|----------|-------|
//! | [`En50160Checker`] | EN 50160 | Voltage magnitude, frequency, THD, flicker |
//! | [`Ieee519Checker`] | IEEE 519-2022 | Harmonic voltage/current limits at PCC |
//! | [`Iec610003_2Checker`] | IEC 61000-3-2 | Equipment-level harmonic current emission |
//! | [`NercTplChecker`] | NERC TPL-001-5 | Transmission planning voltage/loading |
//! | [`PqEventRecorder`] | multi-standard | Event recording, SAIFI/MAIFI indices |
//! | [`ComplianceDashboard`] | aggregate | Overall compliance score |
//!
//! All computation is pure Rust with no `unwrap()` calls.

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// EN 50160 — Supply Voltage Characteristics
// ─────────────────────────────────────────────────────────────────────────────

/// EN 50160 compliance checker for a single monitoring point.
///
/// The standard uses weekly observation windows.  Voltage and THD are sampled
/// as 10-minute RMS values; frequency as 10-second values; flicker as 2-hour
/// long-term severity values (Plt).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct En50160Checker {
    /// Nominal voltage \[kV\] of the supply point.
    pub nominal_voltage_kv: f64,
    /// Nominal system frequency \[Hz\] (50 or 60).
    pub nominal_freq_hz: f64,
    /// Observation window \[days\] (typically 7).
    pub measurement_period_days: u32,
}

/// A single voltage magnitude violation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoltageViolationRecord {
    /// Sample index in the input slice.
    pub index: usize,
    /// Measured voltage \[pu\].
    pub value_pu: f64,
    /// Signed deviation from 1.0 pu \[%\]  (positive = above nominal).
    pub deviation_pct: f64,
}

/// Result of an EN 50160 voltage-magnitude check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoltageComplianceResult {
    /// Percentage of samples within ±10 % of nominal \[%\].
    pub pct_within_10_pct: f64,
    /// Percentage of samples within −15 %/+10 % of nominal \[%\].
    pub pct_within_15_pct: f64,
    /// True when ≥ 95 % of samples are within ±10 %.
    pub compliant_95_pct: bool,
    /// True when 100 % of samples are within −15 %/+10 %.
    pub compliant_100_pct: bool,
    /// Maximum absolute deviation from 1.0 pu across all samples \[%\].
    pub max_deviation_pct: f64,
    /// Individual samples that violate the ±10 % (95 %) limit.
    pub violations: Vec<VoltageViolationRecord>,
}

/// Result of an EN 50160 frequency check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreqComplianceResult {
    /// Percentage of samples within the normal band (±0.5 Hz) \[%\].
    pub pct_in_normal_band: f64,
    /// Percentage of samples within the extended band (47–52 Hz) \[%\].
    pub pct_in_extended_band: f64,
    /// True when ≥ 95 % in normal band AND 100 % in extended band.
    pub compliant: bool,
    /// Maximum absolute deviation from nominal frequency \[Hz\].
    pub max_deviation_hz: f64,
}

/// Result of an EN 50160 THD check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThdComplianceResult {
    /// Percentage of 10-minute THD samples at or below the 8 % limit \[%\].
    pub pct_within_limit: f64,
    /// True when ≥ 95 % of samples satisfy THD ≤ 8 %.
    pub compliant: bool,
    /// Maximum THD observed across all samples \[%\].
    pub max_thd_pct: f64,
}

/// Result of an EN 50160 long-term flicker check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlickerComplianceResult {
    /// Percentage of Plt values at or below 1.0 \[%\].
    pub pct_within_limit: f64,
    /// True when ≥ 95 % of Plt values are ≤ 1.0.
    pub compliant: bool,
    /// Maximum Plt observed across all samples.
    pub max_plt: f64,
}

/// Aggregated EN 50160 compliance report for one monitoring period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct En50160Report {
    /// Voltage magnitude sub-report.
    pub voltage: VoltageComplianceResult,
    /// Frequency sub-report.
    pub frequency: FreqComplianceResult,
    /// THD sub-report.
    pub thd: ThdComplianceResult,
    /// Long-term flicker sub-report.
    pub flicker: FlickerComplianceResult,
    /// True only when all four sub-checks pass.
    pub fully_compliant: bool,
    /// Weighted compliance score 0–100 (equal weighting per sub-check).
    pub compliance_score_pct: f64,
}

impl En50160Checker {
    /// Create a new checker with the given nominal parameters.
    pub fn new(
        nominal_voltage_kv: f64,
        nominal_freq_hz: f64,
        measurement_period_days: u32,
    ) -> Self {
        Self {
            nominal_voltage_kv,
            nominal_freq_hz,
            measurement_period_days,
        }
    }

    /// Check voltage magnitude compliance against EN 50160 §11.
    ///
    /// - 95 % of 10-minute values must lie within ±10 % of nominal.
    /// - 100 % must lie within −15 %/+10 % of nominal.
    pub fn check_voltage_magnitude(&self, v_measurements_pu: &[f64]) -> VoltageComplianceResult {
        let n = v_measurements_pu.len();
        if n == 0 {
            return VoltageComplianceResult {
                pct_within_10_pct: 100.0,
                pct_within_15_pct: 100.0,
                compliant_95_pct: true,
                compliant_100_pct: true,
                max_deviation_pct: 0.0,
                violations: Vec::new(),
            };
        }
        let mut within_10 = 0usize;
        let mut within_15 = 0usize;
        let mut max_dev = 0.0_f64;
        let mut violations = Vec::new();

        for (idx, &v) in v_measurements_pu.iter().enumerate() {
            let dev = (v - 1.0) * 100.0; // signed %
            let abs_dev = dev.abs();
            if abs_dev > max_dev {
                max_dev = abs_dev;
            }
            // ±10 % symmetric limit (95 % rule)
            if abs_dev <= 10.0 {
                within_10 += 1;
            } else {
                violations.push(VoltageViolationRecord {
                    index: idx,
                    value_pu: v,
                    deviation_pct: dev,
                });
            }
            // −15 %/+10 % asymmetric limit (100 % rule)
            if (-15.0..=10.0).contains(&dev) {
                within_15 += 1;
            }
        }

        let pct_within_10 = (within_10 as f64 / n as f64) * 100.0;
        let pct_within_15 = (within_15 as f64 / n as f64) * 100.0;

        VoltageComplianceResult {
            pct_within_10_pct: pct_within_10,
            pct_within_15_pct: pct_within_15,
            compliant_95_pct: pct_within_10 >= 95.0,
            compliant_100_pct: within_15 == n,
            max_deviation_pct: max_dev,
            violations,
        }
    }

    /// Check frequency compliance against EN 50160 §10 (interconnected network).
    ///
    /// - 95 % of 10-second values must lie within ±0.5 Hz of nominal.
    /// - 100 % must lie within 47–52 Hz (for 50 Hz system).
    pub fn check_frequency(&self, freq_measurements: &[f64]) -> FreqComplianceResult {
        let n = freq_measurements.len();
        if n == 0 {
            return FreqComplianceResult {
                pct_in_normal_band: 100.0,
                pct_in_extended_band: 100.0,
                compliant: true,
                max_deviation_hz: 0.0,
            };
        }
        let nominal = self.nominal_freq_hz;
        // Extended band: for 50 Hz system 47..52 Hz; for 60 Hz system 57..62 Hz
        let ext_low = nominal - 3.0;
        let ext_high = nominal + 2.0;

        let mut in_normal = 0usize;
        let mut in_extended = 0usize;
        let mut max_dev = 0.0_f64;

        for &f in freq_measurements {
            let dev = (f - nominal).abs();
            if dev > max_dev {
                max_dev = dev;
            }
            if dev <= 0.5 {
                in_normal += 1;
            }
            if f >= ext_low && f <= ext_high {
                in_extended += 1;
            }
        }

        let pct_normal = (in_normal as f64 / n as f64) * 100.0;
        let pct_extended = (in_extended as f64 / n as f64) * 100.0;

        FreqComplianceResult {
            pct_in_normal_band: pct_normal,
            pct_in_extended_band: pct_extended,
            compliant: pct_normal >= 95.0 && in_extended == n,
            max_deviation_hz: max_dev,
        }
    }

    /// Check voltage THD compliance against EN 50160 §12.
    ///
    /// 95 % of weekly 10-minute THD values must be ≤ 8 \[%\].
    pub fn check_thd(&self, thd_measurements: &[f64]) -> ThdComplianceResult {
        let n = thd_measurements.len();
        if n == 0 {
            return ThdComplianceResult {
                pct_within_limit: 100.0,
                compliant: true,
                max_thd_pct: 0.0,
            };
        }
        let limit = 8.0_f64;
        let within = thd_measurements.iter().filter(|&&t| t <= limit).count();
        let max_thd = thd_measurements
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let pct = (within as f64 / n as f64) * 100.0;
        ThdComplianceResult {
            pct_within_limit: pct,
            compliant: pct >= 95.0,
            max_thd_pct: max_thd,
        }
    }

    /// Check long-term flicker compliance against EN 50160 §13.
    ///
    /// 95 % of weekly 2-hour Plt values must be ≤ 1.0.
    pub fn check_flicker(&self, plt_measurements: &[f64]) -> FlickerComplianceResult {
        let n = plt_measurements.len();
        if n == 0 {
            return FlickerComplianceResult {
                pct_within_limit: 100.0,
                compliant: true,
                max_plt: 0.0,
            };
        }
        let limit = 1.0_f64;
        let within = plt_measurements.iter().filter(|&&p| p <= limit).count();
        let max_plt = plt_measurements
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let pct = (within as f64 / n as f64) * 100.0;
        FlickerComplianceResult {
            pct_within_limit: pct,
            compliant: pct >= 95.0,
            max_plt,
        }
    }

    /// Generate a full EN 50160 compliance report.
    pub fn full_compliance_report(
        &self,
        v: &[f64],
        freq: &[f64],
        thd: &[f64],
        plt: &[f64],
    ) -> En50160Report {
        let voltage = self.check_voltage_magnitude(v);
        let frequency = self.check_frequency(freq);
        let thd_res = self.check_thd(thd);
        let flicker = self.check_flicker(plt);

        let pass_count = [
            voltage.compliant_95_pct && voltage.compliant_100_pct,
            frequency.compliant,
            thd_res.compliant,
            flicker.compliant,
        ]
        .iter()
        .filter(|&&b| b)
        .count();

        let fully_compliant = pass_count == 4;
        let compliance_score_pct = (pass_count as f64 / 4.0) * 100.0;

        En50160Report {
            voltage,
            frequency,
            thd: thd_res,
            flicker,
            fully_compliant,
            compliance_score_pct,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// IEEE 519-2022 — Harmonic Control
// ─────────────────────────────────────────────────────────────────────────────

/// IEEE 519-2022 harmonic compliance checker at the point of common coupling (PCC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ieee519Checker {
    /// PCC bus voltage \[kV\].
    pub pcc_voltage_kv: f64,
    /// Short-circuit to load current ratio (Isc / IL) at the PCC.
    pub isc_il_ratio: f64,
}

/// Result of an IEEE 519 harmonic compliance check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarmonicComplianceResult {
    /// True when both THD and all individual harmonics are within limits.
    pub compliant: bool,
    /// Computed THD from the supplied harmonic list \[%\].
    pub thd_pct: f64,
    /// Applicable THD limit for this PCC \[%\].
    pub thd_limit_pct: f64,
    /// Harmonics that exceed their individual limits: `(order, measured_%, limit_%)`.
    pub exceeding_harmonics: Vec<(usize, f64, f64)>,
}

impl Ieee519Checker {
    /// Create a new checker for the specified PCC.
    pub fn new(pcc_voltage_kv: f64, isc_il_ratio: f64) -> Self {
        Self {
            pcc_voltage_kv,
            isc_il_ratio,
        }
    }

    /// Total harmonic distortion limit for voltage at this PCC \[%\].
    ///
    /// IEEE 519-2022 Table 1:
    /// - < 1 kV → 8 %
    /// - 1–69 kV → 5 %
    /// - 69–161 kV → 2.5 %
    /// - > 161 kV → 1.5 %
    pub fn voltage_thd_limit_pct(&self) -> f64 {
        let kv = self.pcc_voltage_kv;
        if kv < 1.0 {
            8.0
        } else if kv < 69.0 {
            5.0
        } else if kv < 161.0 {
            2.5
        } else {
            1.5
        }
    }

    /// Total demand distortion (TDD) limit for current at this PCC \[%\].
    ///
    /// IEEE 519-2022 Table 2 — depends on Isc / IL ratio.
    pub fn current_thd_limit_pct(&self) -> f64 {
        let r = self.isc_il_ratio;
        if r < 20.0 {
            5.0
        } else if r < 50.0 {
            8.0
        } else if r < 100.0 {
            12.0
        } else if r < 1000.0 {
            15.0
        } else {
            20.0
        }
    }

    /// Individual voltage harmonic limit at this PCC \[%\].
    ///
    /// IEEE 519-2022 Table 1 — scales with voltage level (same tiers as THD).
    pub fn individual_harmonic_voltage_limit_pct(&self, harmonic_order: usize) -> f64 {
        // The individual limits are the same tier as the THD limit for voltage.
        // IEEE 519 notes that individual harmonic limits are typically 3 % for
        // the dominant harmonics (3rd, 5th, 7th) at 1-69 kV; higher orders
        // taper off.  We use a simplified order-based taper.
        let base = self.voltage_thd_limit_pct();
        // Higher orders are attenuated relative to the base THD limit.
        let taper = match harmonic_order {
            1 => 1.0,
            2..=10 => 0.6,
            11..=20 => 0.4,
            21..=35 => 0.3,
            _ => 0.2,
        };
        base * taper
    }

    /// Individual current harmonic limit at this PCC \[%\] of IL.
    ///
    /// IEEE 519-2022 Table 2 — depends on Isc/IL and harmonic order.
    pub fn individual_harmonic_current_limit_pct(&self, harmonic_order: usize) -> f64 {
        let tdd = self.current_thd_limit_pct();
        // Higher-order harmonics have tighter individual limits.
        let factor = match harmonic_order {
            1 => 1.0,
            2..=10 => 0.7,
            11..=16 => 0.35,
            17..=22 => 0.15,
            _ => 0.10,
        };
        tdd * factor
    }

    /// Check a set of voltage harmonic magnitudes against IEEE 519-2022 limits.
    ///
    /// `harmonic_voltages_pct` is a slice of `(order, magnitude_%)` pairs.
    pub fn check_voltage_harmonics(
        &self,
        harmonic_voltages_pct: &[(usize, f64)],
    ) -> HarmonicComplianceResult {
        let thd_limit = self.voltage_thd_limit_pct();
        let sum_sq: f64 = harmonic_voltages_pct
            .iter()
            .filter(|&&(order, _)| order > 1)
            .map(|&(_, mag)| mag * mag)
            .sum();
        let thd_pct = sum_sq.sqrt();

        let mut exceeding = Vec::new();
        for &(order, mag) in harmonic_voltages_pct {
            if order <= 1 {
                continue;
            }
            let limit = self.individual_harmonic_voltage_limit_pct(order);
            if mag > limit {
                exceeding.push((order, mag, limit));
            }
        }

        let compliant = thd_pct <= thd_limit && exceeding.is_empty();
        HarmonicComplianceResult {
            compliant,
            thd_pct,
            thd_limit_pct: thd_limit,
            exceeding_harmonics: exceeding,
        }
    }

    /// Check a set of current harmonic magnitudes against IEEE 519-2022 limits.
    ///
    /// `harmonic_currents_pct` is a slice of `(order, magnitude_%)` pairs.
    pub fn check_current_harmonics(
        &self,
        harmonic_currents_pct: &[(usize, f64)],
    ) -> HarmonicComplianceResult {
        let thd_limit = self.current_thd_limit_pct();
        let sum_sq: f64 = harmonic_currents_pct
            .iter()
            .filter(|&&(order, _)| order > 1)
            .map(|&(_, mag)| mag * mag)
            .sum();
        let thd_pct = sum_sq.sqrt();

        let mut exceeding = Vec::new();
        for &(order, mag) in harmonic_currents_pct {
            if order <= 1 {
                continue;
            }
            let limit = self.individual_harmonic_current_limit_pct(order);
            if mag > limit {
                exceeding.push((order, mag, limit));
            }
        }

        let compliant = thd_pct <= thd_limit && exceeding.is_empty();
        HarmonicComplianceResult {
            compliant,
            thd_pct,
            thd_limit_pct: thd_limit,
            exceeding_harmonics: exceeding,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// IEC 61000-3-2 — Equipment Harmonic Current Emission Limits
// ─────────────────────────────────────────────────────────────────────────────

/// IEC 61000-3-2 equipment class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipmentClass {
    /// Class A — balanced 3-phase equipment, household appliances.  Reference class.
    ClassA,
    /// Class B — portable tools (higher limits than Class A).
    ClassB,
    /// Class C — lighting equipment.
    ClassC,
    /// Class D — equipment with special waveshape (PCs, TVs, input current < 16 A).
    ClassD,
}

/// IEC 61000-3-2 harmonic current compliance checker for a single piece of equipment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Iec610003_2Checker {
    /// IEC 61000-3-2 equipment class.
    pub equipment_class: EquipmentClass,
    /// Rated input power \[W\].
    pub rated_power_w: f64,
}

/// Result of an IEC 61000-3-2 harmonic current check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Iec610003_2Result {
    /// True when all measured harmonic currents are within the class limits.
    pub compliant: bool,
    /// Harmonics that exceed their limit: `(order, measured_A, limit_A)`.
    pub exceeding_harmonics: Vec<(usize, f64, f64)>,
    /// Sum of all harmonic currents (orders 2 and above) \[A\].
    pub total_harmonic_current_a: f64,
}

impl Iec610003_2Checker {
    /// Create a new checker.
    pub fn new(equipment_class: EquipmentClass, rated_power_w: f64) -> Self {
        Self {
            equipment_class,
            rated_power_w,
        }
    }

    /// Return absolute harmonic current limits \[A\] for each harmonic order.
    ///
    /// Table 1 (Class A), Table 2 (Class B = 1.5 × Class A),
    /// Table 3 (Class C — percentage of fundamental), Table 4 (Class D — mA/W).
    ///
    /// Only harmonics with defined limits are returned.  Class D limits scale
    /// with `rated_power_w`.
    pub fn current_limits_a(&self) -> Vec<(usize, f64)> {
        match self.equipment_class {
            EquipmentClass::ClassA => vec![
                (2, 1.08),
                (3, 2.30),
                (4, 0.43),
                (5, 1.14),
                (6, 0.30),
                (7, 0.77),
                (8, 0.23),
                (9, 0.40),
                (10, 0.184),
                (11, 0.33),
                (12, 0.153),
                (13, 0.21),
                (14, 0.131),
                (15, 0.15),
                (16, 0.115),
                (17, 0.132),
                (19, 0.118),
                (21, 0.107),
                (23, 0.098),
                (25, 0.090),
            ],
            EquipmentClass::ClassB => {
                // Class B limits = 1.5 × Class A
                let class_a = Iec610003_2Checker::new(EquipmentClass::ClassA, self.rated_power_w)
                    .current_limits_a();
                class_a
                    .into_iter()
                    .map(|(order, lim)| (order, lim * 1.5))
                    .collect()
            }
            EquipmentClass::ClassC => {
                // Class C: THD ≤ 30 %, 3rd harmonic ≤ 30 % × power-factor of fundamental
                // Expressed as absolute A at rated 230 V; we use a simplified 230 V base.
                // Odd harmonics only.  Typical rated current ≈ P / (230 × pf).
                let pf = 0.95_f64; // assumed PF
                let i_fund = self.rated_power_w / (230.0 * pf);
                vec![
                    (2, 0.01 * i_fund),
                    (3, 0.30 * pf * i_fund),
                    (5, 0.10 * i_fund),
                    (7, 0.07 * i_fund),
                    (9, 0.05 * i_fund),
                    (11, 0.03 * i_fund),
                    (13, 0.03 * i_fund),
                ]
            }
            EquipmentClass::ClassD => {
                // Class D: limits in mA/W, scaled by rated power.
                let p_w = self.rated_power_w.min(600.0); // cap at 600 W per std
                vec![
                    (3, 3.4e-3 * p_w),
                    (5, 1.9e-3 * p_w),
                    (7, 1.0e-3 * p_w),
                    (9, 0.5e-3 * p_w),
                    (11, 0.35e-3 * p_w),
                ]
            }
        }
    }

    /// Allowed THD \[%\] for this equipment class (informative).
    pub fn allowed_thd_pct(&self) -> f64 {
        match self.equipment_class {
            EquipmentClass::ClassA => 48.0, // derived from Table 1 sum
            EquipmentClass::ClassB => 48.0 * 1.5,
            EquipmentClass::ClassC => 30.0,
            EquipmentClass::ClassD => 75.0,
        }
    }

    /// Check measured harmonic currents against IEC 61000-3-2 limits.
    pub fn check_compliance(&self, harmonic_currents_a: &[(usize, f64)]) -> Iec610003_2Result {
        let limits = self.current_limits_a();
        let mut exceeding = Vec::new();
        let mut total_harmonic = 0.0_f64;

        for &(order, measured) in harmonic_currents_a {
            if order < 2 {
                continue;
            }
            total_harmonic += measured;
            // Find limit for this order (if defined)
            if let Some(&(_, limit)) = limits.iter().find(|&&(o, _)| o == order) {
                if measured > limit {
                    exceeding.push((order, measured, limit));
                }
            }
        }

        Iec610003_2Result {
            compliant: exceeding.is_empty(),
            exceeding_harmonics: exceeding,
            total_harmonic_current_a: total_harmonic,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// NERC TPL — Transmission Planning Standards
// ─────────────────────────────────────────────────────────────────────────────

/// NERC TPL-001-5 planning event category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TplEventType {
    /// TPL-001 — No contingency (normal system).
    Tpl001,
    /// TPL-002 — N-1 single contingency (single element loss).
    Tpl002,
    /// TPL-003 — N-1-1 delayed fault clearing / stuck breaker.
    Tpl003,
    /// TPL-004 — N-2 multiple contingency.
    Tpl004,
    /// TPL-007 — Extreme events (natural disasters, EMP, etc.).
    Tpl007,
}

/// NERC TPL compliance checker for a single planning event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NercTplChecker {
    /// Planning event category.
    pub planning_event_type: TplEventType,
    /// Base bus voltage \[kV\].
    pub base_voltage_kv: f64,
}

/// Result of a NERC TPL contingency check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TplComplianceResult {
    /// True when no voltage or loading violations are found.
    pub compliant: bool,
    /// Bus indices and voltages \[pu\] that violate the applicable limit.
    pub voltage_violations: Vec<(usize, f64)>,
    /// Branch indices and loadings \[%\] that exceed the applicable rating.
    pub loading_violations: Vec<(usize, f64)>,
    /// True when transient or voltage stability studies are required.
    pub stability_required: bool,
}

/// Stability study requirements for a given TPL event level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityCriteria {
    /// Transient stability study required.
    pub transient_stability_required: bool,
    /// Voltage stability (P-V/Q-V) study required.
    pub voltage_stability_required: bool,
    /// Small-signal oscillation damping study required.
    pub oscillation_damping_required: bool,
}

impl NercTplChecker {
    /// Create a new checker for the given TPL category and voltage base.
    pub fn new(planning_event_type: TplEventType, base_voltage_kv: f64) -> Self {
        Self {
            planning_event_type,
            base_voltage_kv,
        }
    }

    /// Allowable post-contingency voltage range \[pu\] per NERC TPL category.
    ///
    /// Returns `(v_min, v_max)`:
    /// - TPL-001: (0.95, 1.05)
    /// - TPL-002: (0.90, 1.10)
    /// - TPL-003: (0.85, 1.10)
    /// - TPL-004: (0.80, 1.10)
    /// - TPL-007: (0.75, 1.10)
    pub fn voltage_limit_pu(&self) -> (f64, f64) {
        match self.planning_event_type {
            TplEventType::Tpl001 => (0.95, 1.05),
            TplEventType::Tpl002 => (0.90, 1.10),
            TplEventType::Tpl003 => (0.85, 1.10),
            TplEventType::Tpl004 => (0.80, 1.10),
            TplEventType::Tpl007 => (0.75, 1.10),
        }
    }

    /// Allowable branch loading \[%\] of normal rating per NERC TPL category.
    ///
    /// - TPL-001/002: 100 % (normal rating)
    /// - TPL-003: 125 % (emergency rating)
    /// - TPL-004/007: 150 % (extended emergency)
    pub fn loading_limit_pct(&self) -> f64 {
        match self.planning_event_type {
            TplEventType::Tpl001 | TplEventType::Tpl002 => 100.0,
            TplEventType::Tpl003 => 125.0,
            TplEventType::Tpl004 | TplEventType::Tpl007 => 150.0,
        }
    }

    /// Evaluate post-contingency bus voltages and branch loadings.
    pub fn check_contingency(
        &self,
        bus_voltages_pu: &[f64],
        branch_loadings_pct: &[f64],
    ) -> TplComplianceResult {
        let (v_min, v_max) = self.voltage_limit_pu();
        let l_limit = self.loading_limit_pct();

        let voltage_violations: Vec<(usize, f64)> = bus_voltages_pu
            .iter()
            .enumerate()
            .filter(|&(_, &v)| v < v_min || v > v_max)
            .map(|(i, &v)| (i, v))
            .collect();

        let loading_violations: Vec<(usize, f64)> = branch_loadings_pct
            .iter()
            .enumerate()
            .filter(|&(_, &l)| l > l_limit)
            .map(|(i, &l)| (i, l))
            .collect();

        let stability_required = self.stability_criteria().transient_stability_required
            || self.stability_criteria().voltage_stability_required;

        let compliant = voltage_violations.is_empty() && loading_violations.is_empty();

        TplComplianceResult {
            compliant,
            voltage_violations,
            loading_violations,
            stability_required,
        }
    }

    /// Return the applicable stability study requirements for this TPL event.
    pub fn stability_criteria(&self) -> StabilityCriteria {
        match self.planning_event_type {
            TplEventType::Tpl001 => StabilityCriteria {
                transient_stability_required: false,
                voltage_stability_required: false,
                oscillation_damping_required: false,
            },
            TplEventType::Tpl002 => StabilityCriteria {
                transient_stability_required: true,
                voltage_stability_required: false,
                oscillation_damping_required: false,
            },
            TplEventType::Tpl003 => StabilityCriteria {
                transient_stability_required: true,
                voltage_stability_required: true,
                oscillation_damping_required: false,
            },
            TplEventType::Tpl004 => StabilityCriteria {
                transient_stability_required: true,
                voltage_stability_required: true,
                oscillation_damping_required: true,
            },
            TplEventType::Tpl007 => StabilityCriteria {
                transient_stability_required: true,
                voltage_stability_required: true,
                oscillation_damping_required: true,
            },
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PQ Event Recorder
// ─────────────────────────────────────────────────────────────────────────────

/// Power quality event type for compliance recording.
///
/// Named `PqEventType2` to avoid colliding with [`crate::powerquality::events::PqEventClass`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PqEventType2 {
    /// Voltage sag (dip) below 0.9 pu.
    VoltageSag,
    /// Voltage swell above 1.1 pu.
    VoltageSwell,
    /// Supply interruption (< 0.1 pu).
    Interruption,
    /// Fast transient (impulsive or oscillatory).
    Transient,
    /// Repetitive voltage fluctuation causing flicker.
    Flicker,
    /// Elevated total harmonic distortion.
    HarmonicDistortion,
    /// System frequency deviation beyond limits.
    FrequencyDeviation,
    /// Three-phase voltage magnitude imbalance.
    VoltageImbalance,
}

impl PqEventType2 {
    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            PqEventType2::VoltageSag => "Voltage Sag",
            PqEventType2::VoltageSwell => "Voltage Swell",
            PqEventType2::Interruption => "Interruption",
            PqEventType2::Transient => "Transient",
            PqEventType2::Flicker => "Flicker",
            PqEventType2::HarmonicDistortion => "Harmonic Distortion",
            PqEventType2::FrequencyDeviation => "Frequency Deviation",
            PqEventType2::VoltageImbalance => "Voltage Imbalance",
        }
    }
}

/// A single recorded power quality compliance event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqComplianceEvent {
    /// Unix timestamp of event onset \[s\].
    pub timestamp: f64,
    /// Classification of this event.
    pub event_type: PqEventType2,
    /// Event magnitude \[pu\] (e.g., retained voltage for sag/swell).
    pub magnitude_pu: f64,
    /// Event duration \[s\].
    pub duration_s: f64,
    /// Energy index V² × t or I² × t \[pu²·s\].
    pub energy_index: f64,
    /// True when the event is within applicable standard limits.
    pub compliant: bool,
    /// Name of the standard used for classification (e.g., "EN 50160").
    pub standard: String,
}

/// Chronological recorder of power quality compliance events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqEventRecorder {
    /// All recorded events in insertion order.
    pub events: Vec<PqComplianceEvent>,
    /// Identifier for the monitoring location (e.g., bus name or meter ID).
    pub location: String,
    /// Unix timestamp when monitoring started \[s\].
    pub start_timestamp: f64,
}

impl PqEventRecorder {
    /// Create a new, empty recorder for the given location.
    pub fn new(location: &str) -> Self {
        Self {
            events: Vec::new(),
            location: location.to_owned(),
            start_timestamp: 0.0,
        }
    }

    /// Append a new event to the recorder.
    pub fn record(&mut self, event: PqComplianceEvent) {
        self.events.push(event);
    }

    /// Count events grouped by event type.
    ///
    /// Returns a vector of `(event_type, count)` pairs, sorted by descending count.
    pub fn count_by_type(&self) -> Vec<(PqEventType2, usize)> {
        use std::collections::HashMap;
        let mut map: HashMap<String, (PqEventType2, usize)> = HashMap::new();
        for ev in &self.events {
            let key = ev.event_type.label().to_owned();
            let entry = map.entry(key).or_insert_with(|| (ev.event_type.clone(), 0));
            entry.1 += 1;
        }
        let mut counts: Vec<(PqEventType2, usize)> = map.into_values().collect();
        counts.sort_by_key(|b| std::cmp::Reverse(b.1));
        counts
    }

    /// Return references to all events whose onset timestamp falls within
    /// `[start, end]` \[s\].
    pub fn events_in_window(&self, start: f64, end: f64) -> Vec<&PqComplianceEvent> {
        self.events
            .iter()
            .filter(|ev| ev.timestamp >= start && ev.timestamp <= end)
            .collect()
    }

    /// System Average Interruption Frequency Index (SAIFI).
    ///
    /// `SAIFI = Σ(interruption_customers) / total_customers`
    ///
    /// This call counts one interruption event affecting `interruption_customers`
    /// per interruption recorded in the event log.
    pub fn saifi_index(&self, total_customers: usize, interruption_customers: usize) -> f64 {
        if total_customers == 0 {
            return 0.0;
        }
        let count = self
            .events
            .iter()
            .filter(|ev| ev.event_type == PqEventType2::Interruption)
            .count();
        (count * interruption_customers) as f64 / total_customers as f64
    }

    /// Momentary Average Interruption Frequency Index (MAIFI).
    ///
    /// Counts interruptions with duration < 5 minutes (momentary).
    pub fn maifi_index(&self, total_customers: usize, momentary_customers: usize) -> f64 {
        if total_customers == 0 {
            return 0.0;
        }
        let count = self
            .events
            .iter()
            .filter(|ev| ev.event_type == PqEventType2::Interruption && ev.duration_s < 300.0)
            .count();
        (count * momentary_customers) as f64 / total_customers as f64
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Compliance Dashboard
// ─────────────────────────────────────────────────────────────────────────────

/// Aggregated compliance dashboard combining results from multiple standards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceDashboard {
    /// EN 50160 full compliance report, if available.
    pub en50160: Option<En50160Report>,
    /// IEEE 519-2022 voltage harmonic result, if available.
    pub ieee519_voltage: Option<HarmonicComplianceResult>,
    /// IEEE 519-2022 current harmonic result, if available.
    pub ieee519_current: Option<HarmonicComplianceResult>,
    /// NERC TPL contingency result, if available.
    pub nerc_tpl: Option<TplComplianceResult>,
    /// Total number of PQ events recorded during the monitoring period.
    pub total_pq_events: usize,
    /// Duration of the monitoring period \[days\].
    pub monitoring_period_days: u32,
}

impl ComplianceDashboard {
    /// Create a new, empty dashboard.
    pub fn new(monitoring_period_days: u32) -> Self {
        Self {
            en50160: None,
            ieee519_voltage: None,
            ieee519_current: None,
            nerc_tpl: None,
            total_pq_events: 0,
            monitoring_period_days,
        }
    }

    /// Compute an overall compliance score 0–100 \[%\].
    ///
    /// Each populated sub-check contributes equally to the score.
    /// Unpopulated checks are excluded from the denominator.
    pub fn overall_compliance_score(&self) -> f64 {
        let mut total = 0u32;
        let mut passing = 0u32;

        if let Some(ref r) = self.en50160 {
            total += 1;
            if r.fully_compliant {
                passing += 1;
            }
        }
        if let Some(ref r) = self.ieee519_voltage {
            total += 1;
            if r.compliant {
                passing += 1;
            }
        }
        if let Some(ref r) = self.ieee519_current {
            total += 1;
            if r.compliant {
                passing += 1;
            }
        }
        if let Some(ref r) = self.nerc_tpl {
            total += 1;
            if r.compliant {
                passing += 1;
            }
        }

        if total == 0 {
            return 100.0;
        }
        (passing as f64 / total as f64) * 100.0
    }

    /// Return the names of standards that have at least one non-compliant result.
    pub fn non_compliant_standards(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(ref r) = self.en50160 {
            if !r.fully_compliant {
                out.push("EN 50160".to_owned());
            }
        }
        if let Some(ref r) = self.ieee519_voltage {
            if !r.compliant {
                out.push("IEEE 519-2022 (voltage harmonics)".to_owned());
            }
        }
        if let Some(ref r) = self.ieee519_current {
            if !r.compliant {
                out.push("IEEE 519-2022 (current harmonics)".to_owned());
            }
        }
        if let Some(ref r) = self.nerc_tpl {
            if !r.compliant {
                out.push("NERC TPL".to_owned());
            }
        }
        out
    }

    /// Generate a human-readable one-line summary.
    pub fn summary_text(&self) -> String {
        let score = self.overall_compliance_score();
        let non_comp = self.non_compliant_standards();
        if non_comp.is_empty() {
            format!(
                "COMPLIANT — score {:.1}% over {} day(s); {} PQ event(s) recorded.",
                score, self.monitoring_period_days, self.total_pq_events
            )
        } else {
            format!(
                "NON-COMPLIANT — score {:.1}%; failing: {}; {} PQ event(s) recorded.",
                score,
                non_comp.join(", "),
                self.total_pq_events
            )
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── EN 50160 tests ──────────────────────────────────────────────────────

    #[test]
    fn en50160_voltage_all_within_10pct_compliant() {
        let checker = En50160Checker::new(0.4, 50.0, 7);
        // All values exactly at nominal → 100 % within ±10 %
        let v: Vec<f64> = vec![1.0; 1008]; // 7 days × 144 samples/day
        let result = checker.check_voltage_magnitude(&v);
        assert!(result.compliant_95_pct, "should be 95%-compliant");
        assert!(result.compliant_100_pct, "should be 100%-compliant");
        assert!(result.violations.is_empty());
    }

    #[test]
    fn en50160_voltage_too_many_violations_fails_95pct() {
        let checker = En50160Checker::new(0.4, 50.0, 7);
        // 10 % of values at 1.15 pu (beyond ±10 %) — fails 95 % rule
        let mut v = vec![1.0_f64; 900];
        v.extend(std::iter::repeat_n(1.15_f64, 100));
        let result = checker.check_voltage_magnitude(&v);
        assert!(
            !result.compliant_95_pct,
            "10% violations should fail the 95% rule"
        );
        assert_eq!(result.violations.len(), 100);
    }

    #[test]
    fn en50160_voltage_asymmetric_limit() {
        let checker = En50160Checker::new(0.4, 50.0, 7);
        // 0.82 pu is within −15 % but outside ±10 %
        let v = vec![0.82_f64; 100];
        let result = checker.check_voltage_magnitude(&v);
        // compliant_100_pct should be true (−18 % → outside; 0.82 = -18% → fails)
        // 1 - 0.82 = 0.18 = 18% below → outside −15% limit
        assert!(!result.compliant_100_pct, "0.82 pu = -18% fails -15% limit");
    }

    #[test]
    fn en50160_frequency_normal_band_compliant() {
        let checker = En50160Checker::new(0.4, 50.0, 7);
        // All in 49.9–50.1 Hz → well within ±0.5 Hz
        let freq: Vec<f64> = (0..1000).map(|i| 50.0 + (i % 3) as f64 * 0.05).collect();
        let result = checker.check_frequency(&freq);
        assert!(result.compliant, "frequency in normal band should pass");
        assert!(result.pct_in_normal_band >= 95.0);
    }

    #[test]
    fn en50160_frequency_extended_band_violation_fails() {
        let checker = En50160Checker::new(0.4, 50.0, 7);
        // Mix: 97 % in normal, but 3 % at 46 Hz (outside 47-52)
        let mut freq = vec![50.0_f64; 970];
        freq.extend(std::iter::repeat_n(46.0_f64, 30));
        let result = checker.check_frequency(&freq);
        // 100% in extended band is violated
        assert!(!result.compliant, "46 Hz is outside extended band");
        assert!(result.pct_in_extended_band < 100.0);
    }

    #[test]
    fn en50160_thd_all_below_limit_compliant() {
        let checker = En50160Checker::new(0.4, 50.0, 7);
        let thd = vec![3.0_f64; 1000];
        let result = checker.check_thd(&thd);
        assert!(result.compliant);
        assert!((result.max_thd_pct - 3.0).abs() < 1e-9);
    }

    #[test]
    fn en50160_flicker_above_limit_fails() {
        let checker = En50160Checker::new(0.4, 50.0, 7);
        // 10 % of Plt values = 1.5 (above 1.0 limit)
        let mut plt = vec![0.5_f64; 900];
        plt.extend(std::iter::repeat_n(1.5_f64, 100));
        let result = checker.check_flicker(&plt);
        assert!(!result.compliant, "10% above Plt limit should fail");
    }

    // ── IEEE 519 tests ──────────────────────────────────────────────────────

    #[test]
    fn ieee519_voltage_thd_limit_varies_with_voltage() {
        // < 1 kV → 8 %
        assert!((Ieee519Checker::new(0.4, 20.0).voltage_thd_limit_pct() - 8.0).abs() < 1e-9);
        // 1–69 kV → 5 %
        assert!((Ieee519Checker::new(13.8, 20.0).voltage_thd_limit_pct() - 5.0).abs() < 1e-9);
        // 69–161 kV → 2.5 %
        assert!((Ieee519Checker::new(115.0, 20.0).voltage_thd_limit_pct() - 2.5).abs() < 1e-9);
        // > 161 kV → 1.5 %
        assert!((Ieee519Checker::new(345.0, 20.0).voltage_thd_limit_pct() - 1.5).abs() < 1e-9);
    }

    #[test]
    fn ieee519_check_voltage_harmonics_identifies_violations() {
        // 13.8 kV PCC, THD limit = 5 %, individual 2–10 limit = 5*0.6 = 3 %
        let checker = Ieee519Checker::new(13.8, 20.0);
        // 5th harmonic at 4 % exceeds 3 % individual limit
        let harmonics = vec![(5, 4.0_f64), (7, 1.0_f64)];
        let result = checker.check_voltage_harmonics(&harmonics);
        assert!(!result.compliant);
        assert_eq!(result.exceeding_harmonics.len(), 1);
        assert_eq!(result.exceeding_harmonics[0].0, 5);
    }

    #[test]
    fn ieee519_thd_computed_correctly() {
        let checker = Ieee519Checker::new(0.4, 20.0);
        // THD = sqrt(3^2 + 4^2) = 5 %
        let harmonics = vec![(3, 3.0_f64), (5, 4.0_f64)];
        let result = checker.check_voltage_harmonics(&harmonics);
        assert!((result.thd_pct - 5.0).abs() < 1e-6);
    }

    // ── IEC 61000-3-2 tests ─────────────────────────────────────────────────

    #[test]
    fn iec61000_class_a_third_harmonic_limit_is_2_3a() {
        let checker = Iec610003_2Checker::new(EquipmentClass::ClassA, 1000.0);
        let limits = checker.current_limits_a();
        let third = limits.iter().find(|&&(o, _)| o == 3);
        assert!(third.is_some(), "3rd harmonic limit should be defined");
        let (_, lim) = third.copied().unwrap_or((0, 0.0));
        assert!(
            (lim - 2.30).abs() < 1e-6,
            "3rd harmonic = 2.30 A for Class A"
        );
    }

    #[test]
    fn iec61000_check_compliance_marks_exceeding_harmonics() {
        let checker = Iec610003_2Checker::new(EquipmentClass::ClassA, 1000.0);
        // 3rd harmonic at 3.0 A exceeds 2.30 A limit
        let measured = vec![(3usize, 3.0_f64), (5, 0.5_f64)];
        let result = checker.check_compliance(&measured);
        assert!(!result.compliant);
        assert_eq!(result.exceeding_harmonics.len(), 1);
        assert_eq!(result.exceeding_harmonics[0].0, 3);
    }

    #[test]
    fn iec61000_class_b_limits_are_1_5x_class_a() {
        let a = Iec610003_2Checker::new(EquipmentClass::ClassA, 1000.0).current_limits_a();
        let b = Iec610003_2Checker::new(EquipmentClass::ClassB, 1000.0).current_limits_a();
        assert_eq!(a.len(), b.len());
        for ((oa, la), (ob, lb)) in a.iter().zip(b.iter()) {
            assert_eq!(oa, ob);
            assert!((lb - la * 1.5).abs() < 1e-6, "Class B = 1.5 × Class A");
        }
    }

    // ── NERC TPL tests ──────────────────────────────────────────────────────

    #[test]
    fn nerc_tpl001_limits_tighter_than_tpl002() {
        let t1 = NercTplChecker::new(TplEventType::Tpl001, 345.0);
        let t2 = NercTplChecker::new(TplEventType::Tpl002, 345.0);
        let (v1_min, v1_max) = t1.voltage_limit_pu();
        let (v2_min, v2_max) = t2.voltage_limit_pu();
        assert!(v1_min > v2_min, "TPL-001 lower bound tighter than TPL-002");
        assert!(v1_max < v2_max, "TPL-001 upper bound tighter than TPL-002");
    }

    #[test]
    fn nerc_tpl_check_contingency_finds_voltage_violations() {
        let checker = NercTplChecker::new(TplEventType::Tpl001, 345.0);
        // Bus 0 at 0.93 pu violates TPL-001 lower limit of 0.95
        let voltages = vec![0.93_f64, 1.0, 1.02];
        let loadings = vec![80.0_f64, 70.0];
        let result = checker.check_contingency(&voltages, &loadings);
        assert!(!result.compliant);
        assert_eq!(result.voltage_violations.len(), 1);
        assert_eq!(result.voltage_violations[0].0, 0);
    }

    #[test]
    fn nerc_tpl_check_contingency_finds_loading_violations() {
        let checker = NercTplChecker::new(TplEventType::Tpl001, 345.0);
        let voltages = vec![1.0_f64; 3];
        // Branch 1 at 110 % exceeds 100 % limit for TPL-001
        let loadings = vec![80.0_f64, 110.0];
        let result = checker.check_contingency(&voltages, &loadings);
        assert!(!result.compliant);
        assert_eq!(result.loading_violations.len(), 1);
        assert_eq!(result.loading_violations[0].0, 1);
    }

    // ── PqEventRecorder tests ───────────────────────────────────────────────

    fn make_event(ts: f64, event_type: PqEventType2) -> PqComplianceEvent {
        PqComplianceEvent {
            timestamp: ts,
            event_type,
            magnitude_pu: 0.85,
            duration_s: 0.1,
            energy_index: 0.85 * 0.85 * 0.1,
            compliant: false,
            standard: "EN 50160".to_owned(),
        }
    }

    #[test]
    fn pq_recorder_count_by_type_groups_correctly() {
        let mut rec = PqEventRecorder::new("Bus-1");
        rec.record(make_event(0.0, PqEventType2::VoltageSag));
        rec.record(make_event(1.0, PqEventType2::VoltageSag));
        rec.record(make_event(2.0, PqEventType2::Interruption));
        let counts = rec.count_by_type();
        let sag_count = counts
            .iter()
            .find(|(t, _)| *t == PqEventType2::VoltageSag)
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let int_count = counts
            .iter()
            .find(|(t, _)| *t == PqEventType2::Interruption)
            .map(|(_, c)| *c)
            .unwrap_or(0);
        assert_eq!(sag_count, 2);
        assert_eq!(int_count, 1);
    }

    #[test]
    fn pq_recorder_events_in_window_filters_correctly() {
        let mut rec = PqEventRecorder::new("Bus-2");
        rec.record(make_event(10.0, PqEventType2::VoltageSag));
        rec.record(make_event(20.0, PqEventType2::VoltageSwell));
        rec.record(make_event(30.0, PqEventType2::Flicker));
        let window = rec.events_in_window(15.0, 25.0);
        assert_eq!(window.len(), 1);
        assert_eq!(window[0].event_type, PqEventType2::VoltageSwell);
    }

    #[test]
    fn pq_recorder_saifi_computes_correctly() {
        let mut rec = PqEventRecorder::new("Feeder-A");
        rec.record(make_event(0.0, PqEventType2::Interruption));
        rec.record(make_event(1.0, PqEventType2::Interruption));
        rec.record(make_event(2.0, PqEventType2::VoltageSag));
        // 2 interruptions × 50 customers / 1000 = 0.1
        let saifi = rec.saifi_index(1000, 50);
        assert!((saifi - 0.1).abs() < 1e-9);
    }

    // ── ComplianceDashboard tests ───────────────────────────────────────────

    #[test]
    fn dashboard_overall_score_100_when_all_compliant() {
        let mut dash = ComplianceDashboard::new(7);
        // EN 50160 fully compliant
        dash.en50160 = Some(En50160Report {
            voltage: VoltageComplianceResult {
                pct_within_10_pct: 100.0,
                pct_within_15_pct: 100.0,
                compliant_95_pct: true,
                compliant_100_pct: true,
                max_deviation_pct: 0.0,
                violations: vec![],
            },
            frequency: FreqComplianceResult {
                pct_in_normal_band: 100.0,
                pct_in_extended_band: 100.0,
                compliant: true,
                max_deviation_hz: 0.0,
            },
            thd: ThdComplianceResult {
                pct_within_limit: 100.0,
                compliant: true,
                max_thd_pct: 2.0,
            },
            flicker: FlickerComplianceResult {
                pct_within_limit: 100.0,
                compliant: true,
                max_plt: 0.5,
            },
            fully_compliant: true,
            compliance_score_pct: 100.0,
        });
        // IEEE 519 voltage compliant
        dash.ieee519_voltage = Some(HarmonicComplianceResult {
            compliant: true,
            thd_pct: 2.0,
            thd_limit_pct: 5.0,
            exceeding_harmonics: vec![],
        });
        // IEEE 519 current compliant
        dash.ieee519_current = Some(HarmonicComplianceResult {
            compliant: true,
            thd_pct: 3.0,
            thd_limit_pct: 8.0,
            exceeding_harmonics: vec![],
        });
        // NERC TPL compliant
        dash.nerc_tpl = Some(TplComplianceResult {
            compliant: true,
            voltage_violations: vec![],
            loading_violations: vec![],
            stability_required: false,
        });

        let score = dash.overall_compliance_score();
        assert!((score - 100.0).abs() < 1e-9, "score should be 100.0");
        assert!(dash.non_compliant_standards().is_empty());
        assert!(dash.summary_text().starts_with("COMPLIANT"));
    }

    #[test]
    fn dashboard_non_compliant_standards_listed() {
        let mut dash = ComplianceDashboard::new(7);
        dash.ieee519_voltage = Some(HarmonicComplianceResult {
            compliant: false,
            thd_pct: 6.0,
            thd_limit_pct: 5.0,
            exceeding_harmonics: vec![(5, 4.0, 3.0)],
        });
        let nc = dash.non_compliant_standards();
        assert_eq!(nc.len(), 1);
        assert!(nc[0].contains("IEEE 519"));
        let score = dash.overall_compliance_score();
        assert!((score - 0.0).abs() < 1e-9);
    }

    // ── 8 new tests ───────────────────────────────────────────────────────────

    #[test]
    fn en50160_full_compliance_report_all_good() {
        let checker = En50160Checker::new(0.4, 50.0, 7);
        let v = vec![1.0_f64; 100];
        let freq = vec![50.0_f64; 100];
        let thd = vec![3.0_f64; 100];
        let plt = vec![0.5_f64; 100];
        let report = checker.full_compliance_report(&v, &freq, &thd, &plt);
        assert!(
            report.fully_compliant,
            "all-good inputs should be fully compliant"
        );
        assert!((report.compliance_score_pct - 100.0).abs() < 1e-9);
    }

    #[test]
    fn en50160_full_compliance_report_partial_score() {
        let checker = En50160Checker::new(0.4, 50.0, 7);
        let v = vec![1.0_f64; 100];
        let freq = vec![50.0_f64; 100];
        // THD too high → fails THD check
        let thd = vec![10.0_f64; 100];
        let plt = vec![0.5_f64; 100];
        let report = checker.full_compliance_report(&v, &freq, &thd, &plt);
        assert!(
            !report.fully_compliant,
            "high THD should cause non-compliance"
        );
        // 3 out of 4 checks pass → 75%
        assert!((report.compliance_score_pct - 75.0).abs() < 1e-9);
    }

    #[test]
    fn ieee519_current_thd_limit_varies_with_isc_il_ratio() {
        // r < 20 → 5%
        assert!((Ieee519Checker::new(13.8, 10.0).current_thd_limit_pct() - 5.0).abs() < 1e-9);
        // 20 ≤ r < 50 → 8%
        assert!((Ieee519Checker::new(13.8, 25.0).current_thd_limit_pct() - 8.0).abs() < 1e-9);
        // 100 ≤ r < 1000 → 15%
        assert!((Ieee519Checker::new(13.8, 500.0).current_thd_limit_pct() - 15.0).abs() < 1e-9);
        // r ≥ 1000 → 20%
        assert!((Ieee519Checker::new(13.8, 1000.0).current_thd_limit_pct() - 20.0).abs() < 1e-9);
    }

    #[test]
    fn ieee519_check_current_harmonics_compliant_for_small_values() {
        let checker = Ieee519Checker::new(13.8, 10.0); // TDD limit = 5 %
                                                       // THD = sqrt(1^2 + 1^2) ≈ 1.41 %, well below 5 %
        let harmonics = vec![(3usize, 1.0_f64), (5, 1.0_f64)];
        let result = checker.check_current_harmonics(&harmonics);
        assert!(
            result.compliant,
            "small harmonics should pass current check"
        );
        assert!(
            result.exceeding_harmonics.is_empty(),
            "no individual violations expected"
        );
    }

    #[test]
    fn ieee519_individual_harmonic_voltage_limit_tapers_with_order() {
        let checker = Ieee519Checker::new(13.8, 20.0); // THD limit = 5%
                                                       // orders 2–10 → base 5 * 0.6 = 3.0%
        let lim_5th = checker.individual_harmonic_voltage_limit_pct(5);
        assert!(
            (lim_5th - 3.0).abs() < 1e-9,
            "5th limit should be 3.0%, got {lim_5th}"
        );
        // orders 11–20 → 5 * 0.4 = 2.0%
        let lim_13th = checker.individual_harmonic_voltage_limit_pct(13);
        assert!(
            (lim_13th - 2.0).abs() < 1e-9,
            "13th limit should be 2.0%, got {lim_13th}"
        );
        // higher limit for lower order
        assert!(
            lim_5th > lim_13th,
            "lower-order harmonics should have higher limits"
        );
    }

    #[test]
    fn nerc_tpl_loading_limit_increases_with_severity() {
        let t1 = NercTplChecker::new(TplEventType::Tpl001, 345.0);
        let t3 = NercTplChecker::new(TplEventType::Tpl003, 345.0);
        let t4 = NercTplChecker::new(TplEventType::Tpl004, 345.0);
        assert!((t1.loading_limit_pct() - 100.0).abs() < 1e-9);
        assert!((t3.loading_limit_pct() - 125.0).abs() < 1e-9);
        assert!((t4.loading_limit_pct() - 150.0).abs() < 1e-9);
    }

    #[test]
    fn nerc_tpl_stability_criteria_escalates_with_tpl_level() {
        let t1 = NercTplChecker::new(TplEventType::Tpl001, 345.0).stability_criteria();
        let t2 = NercTplChecker::new(TplEventType::Tpl002, 345.0).stability_criteria();
        let t4 = NercTplChecker::new(TplEventType::Tpl004, 345.0).stability_criteria();
        assert!(
            !t1.transient_stability_required,
            "TPL-001 needs no stability studies"
        );
        assert!(
            t2.transient_stability_required,
            "TPL-002 requires transient stability"
        );
        assert!(
            t4.oscillation_damping_required,
            "TPL-004 requires oscillation damping"
        );
    }

    #[test]
    fn pq_recorder_maifi_counts_only_momentary_interruptions() {
        let mut rec = PqEventRecorder::new("Feeder-B");
        // 2 momentary interruptions (< 5 min = 300 s)
        let mut ev1 = make_event(0.0, PqEventType2::Interruption);
        ev1.duration_s = 60.0;
        let mut ev2 = make_event(1.0, PqEventType2::Interruption);
        ev2.duration_s = 120.0;
        // 1 sustained interruption (> 5 min)
        let mut ev3 = make_event(2.0, PqEventType2::Interruption);
        ev3.duration_s = 600.0;
        rec.record(ev1);
        rec.record(ev2);
        rec.record(ev3);
        // MAIFI should count only the 2 momentary ones
        let maifi = rec.maifi_index(1000, 50);
        // 2 × 50 / 1000 = 0.1
        assert!(
            (maifi - 0.1).abs() < 1e-9,
            "MAIFI should be 0.1, got {maifi}"
        );
    }
}
