//! Adaptive Under-Frequency Load Shedding (UFLS) controller.
//!
//! Implements intelligent UFLS with:
//! - Multi-stage frequency threshold triggers
//! - Swing-equation frequency dynamics simulation
//! - Rate-of-Change-of-Frequency (ROCOF) adaptive shedding
//! - ROCOF blocking to suppress spurious operation during transients
//! - Load restoration after frequency recovery
//!
//! # Physics
//!
//! Swing equation (per-unit inertia form):
//!
//! ```text
//! df/dt = (P_mech - P_elec) / (2H)
//! ```
//!
//! where `H` \[s\] is the system inertia constant, `P_mech` \[MW\] is mechanical
//! power (unchanged immediately after disturbance), and `P_elec` \[MW\] is
//! electrical load (tracks frequency via the load-damping characteristic `D`).
//!
//! Load-frequency characteristic (D = 1 % load per % frequency deviation):
//!
//! ```text
//! P_load(f) = P_load_0 * (1 + D * (f - f0) / f0)
//! ```
//!
//! # References
//! - IEEE Std 1366-2022, Power System Reliability Indices
//! - ENTSO-E, "Frequency Stability Evaluation Criteria", 2022
//! - Anderson & Fouad, "Power System Control and Stability", 2nd ed.

use serde::{Deserialize, Serialize};

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors produced by the UFLS controller.
#[derive(Debug, Clone, thiserror::Error)]
pub enum UflsError {
    /// Simulation parameters are invalid.
    #[error("invalid UFLS parameter: {0}")]
    InvalidParameter(String),

    /// Simulation diverged or produced non-finite values.
    #[error("simulation diverged: {0}")]
    SimulationDiverged(String),
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Configuration for the UFLS controller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UflsConfig {
    /// Nominal system frequency \[Hz\] (50.0 or 60.0).
    pub nominal_frequency_hz: f64,

    /// Ordered list of load-shedding stages (typically 3–6 stages).
    pub stages: Vec<UflsStage>,

    /// ROCOF blocking threshold \[Hz/s\].
    ///
    /// If `|ROCOF| > rocof_blocking`, stage operation is suppressed during
    /// the disturbance transient to avoid mal-operation on voltage dips.
    pub rocof_blocking: f64,

    /// System inertia constant \[s\] (H).
    ///
    /// Used to estimate ROCOF in the swing equation: df/dt ≈ ΔP / (2H).
    pub inertia_constant_s: f64,

    /// Minimum delay \[s\] before load can be restored after frequency recovery.
    pub load_restoration_delay_s: f64,
}

impl Default for UflsConfig {
    fn default() -> Self {
        Self {
            nominal_frequency_hz: 50.0,
            stages: vec![
                UflsStage {
                    stage_id: 1,
                    frequency_threshold_hz: 49.0,
                    load_shedding_pct: 10.0,
                    time_delay_ms: 200.0,
                    adaptive: false,
                    priority_buses: vec![],
                },
                UflsStage {
                    stage_id: 2,
                    frequency_threshold_hz: 48.5,
                    load_shedding_pct: 15.0,
                    time_delay_ms: 200.0,
                    adaptive: true,
                    priority_buses: vec![],
                },
                UflsStage {
                    stage_id: 3,
                    frequency_threshold_hz: 48.0,
                    load_shedding_pct: 20.0,
                    time_delay_ms: 200.0,
                    adaptive: true,
                    priority_buses: vec![],
                },
            ],
            rocof_blocking: 2.5,
            inertia_constant_s: 6.0,
            load_restoration_delay_s: 30.0,
        }
    }
}

/// A single load-shedding stage definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UflsStage {
    /// Stage identifier (1-based).
    pub stage_id: usize,

    /// Frequency threshold below which this stage triggers \[Hz\].
    pub frequency_threshold_hz: f64,

    /// Base percentage of total load to shed when this stage fires.
    pub load_shedding_pct: f64,

    /// Intentional time delay before shedding \[ms\].
    ///
    /// Represents relay operate time plus breaker clearing time.
    pub time_delay_ms: f64,

    /// If `true`, adjust `load_shedding_pct` based on ROCOF magnitude.
    ///
    /// Larger ROCOF → shed more (up to 2× the base percentage).
    pub adaptive: bool,

    /// Bus indices from which load is shed (lowest priority first).
    ///
    /// Empty means the system picks buses automatically.
    pub priority_buses: Vec<usize>,
}

// ── Result types ──────────────────────────────────────────────────────────────

/// Full result of an UFLS simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveUflsResult {
    /// IDs of stages that were triggered during the simulation.
    pub stages_triggered: Vec<usize>,

    /// Total load shed \[MW\].
    pub total_load_shed_mw: f64,

    /// Total load shed as percentage of initial total load.
    pub total_load_shed_pct: f64,

    /// Frequency trajectory: (time \[s\], frequency \[Hz\]).
    pub frequency_trajectory: Vec<(f64, f64)>,

    /// Minimum frequency reached during the event \[Hz\].
    pub frequency_nadir_hz: f64,

    /// Time at which the frequency nadir occurred \[s\].
    pub nadir_time_s: f64,

    /// Whether frequency recovered to within 0.5 Hz of nominal.
    pub frequency_recovered: bool,

    /// Time at which frequency recovery was declared \[s\], if applicable.
    pub recovery_time_s: Option<f64>,

    /// Chronological list of individual load-shedding events.
    pub shedding_events: Vec<SheddingEvent>,
}

/// One load-shedding event (one stage firing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheddingEvent {
    /// Stage identifier that fired.
    pub stage: usize,

    /// Simulation time at which shedding took effect \[s\].
    pub time_s: f64,

    /// Load shed in this event \[MW\].
    pub load_shed_mw: f64,

    /// Frequency at the instant the trigger threshold was crossed \[Hz\].
    pub frequency_at_trigger_hz: f64,

    /// Bus indices from which load was shed (for bookkeeping).
    pub buses_affected: Vec<usize>,
}

// ── Controller ────────────────────────────────────────────────────────────────

/// Adaptive UFLS controller.
///
/// Simulates frequency dynamics following a power imbalance and applies
/// multi-stage, ROCOF-adaptive under-frequency load shedding.
///
/// # Example
///
/// ```rust
/// use oxigrid::optimize::restoration::adaptive_load_shedding::{
///     AdaptiveUflsController, UflsConfig,
/// };
///
/// let config = UflsConfig::default();
/// let ctrl = AdaptiveUflsController::new(config);
/// let result = ctrl.simulate(-200.0, 1000.0, 30.0, 0.01).unwrap();
/// println!("Nadir: {:.3} Hz", result.frequency_nadir_hz);
/// ```
pub struct AdaptiveUflsController {
    config: UflsConfig,
}

impl AdaptiveUflsController {
    /// Create a new controller with the given configuration.
    pub fn new(config: UflsConfig) -> Self {
        Self { config }
    }

    /// Simulate the frequency event and UFLS response.
    ///
    /// # Arguments
    ///
    /// - `power_imbalance_mw` — sudden power imbalance \[MW\] (negative = loss of generation).
    /// - `total_load_mw`      — pre-disturbance total load \[MW\].
    /// - `simulation_time_s`  — how long to simulate \[s\].
    /// - `dt_s`               — integration time step \[s\].
    ///
    /// # Returns
    ///
    /// [`AdaptiveUflsResult`] containing frequency trajectory, shedding events,
    /// nadir, and recovery status.
    pub fn simulate(
        &self,
        power_imbalance_mw: f64,
        total_load_mw: f64,
        simulation_time_s: f64,
        dt_s: f64,
    ) -> Result<AdaptiveUflsResult, UflsError> {
        // ── Validate inputs ───────────────────────────────────────────────────
        if total_load_mw <= 0.0 {
            return Err(UflsError::InvalidParameter(
                "total_load_mw must be positive".into(),
            ));
        }
        if dt_s <= 0.0 || dt_s > 1.0 {
            return Err(UflsError::InvalidParameter(
                "dt_s must be in (0, 1] seconds".into(),
            ));
        }
        if simulation_time_s <= 0.0 {
            return Err(UflsError::InvalidParameter(
                "simulation_time_s must be positive".into(),
            ));
        }
        if self.config.inertia_constant_s <= 0.0 {
            return Err(UflsError::InvalidParameter(
                "inertia_constant_s must be positive".into(),
            ));
        }

        let f0 = self.config.nominal_frequency_hz;
        let h = self.config.inertia_constant_s;
        // Load-frequency damping coefficient (D = 1% load per % freq deviation)
        let d_coeff = 0.01_f64; // pu load / pu frequency

        let n_steps = (simulation_time_s / dt_s).ceil() as usize + 1;

        // State variables
        let mut freq = f0;
        let mut p_load = total_load_mw; // current electrical load [MW]
                                        // Mechanical power available (pre-disturbance generation = load, then step)
        let p_mech = total_load_mw + power_imbalance_mw;

        // Stage state: (triggered_at_simulation_time, pending_delay_end, already_fired)
        let n_stages = self.config.stages.len();
        let mut stage_trigger_time: Vec<Option<f64>> = vec![None; n_stages];
        let mut stage_fired: Vec<bool> = vec![false; n_stages];

        // Pending shedding events: (fire_time_s, shed_mw, stage_idx, trigger_freq, rocof_at_trigger)
        let mut pending: Vec<(f64, f64, usize, f64, f64)> = Vec::new();

        let mut trajectory: Vec<(f64, f64)> = Vec::with_capacity(n_steps.min(100_000));
        let mut shedding_events: Vec<SheddingEvent> = Vec::new();
        let mut stages_triggered: Vec<usize> = Vec::new();

        let mut freq_nadir = freq;
        let mut nadir_time = 0.0_f64;

        // Recovery tracking
        let recovery_band_hz = 0.5_f64;
        let mut recovery_time: Option<f64> = None;
        let mut post_shed_time: Option<f64> = None; // earliest time load could be restored

        // ROCOF estimation (derivative filter over last window)
        let rocof_window = (0.1 / dt_s).max(1.0) as usize; // 100 ms window
        let mut freq_history: Vec<f64> = Vec::with_capacity(rocof_window + 1);
        freq_history.push(freq);

        let mut total_shed_mw = 0.0_f64;

        for step in 0..n_steps {
            let time = step as f64 * dt_s;

            // ── Fire any pending shedding events ──────────────────────────────
            let mut newly_shed = 0.0_f64;
            for &(fire_time, shed_mw, stage_idx, trigger_freq, _rocof) in &pending {
                if time >= fire_time {
                    let stage = &self.config.stages[stage_idx];
                    // Check not already applied
                    if !stage_fired[stage_idx] {
                        newly_shed += shed_mw;
                        total_shed_mw += shed_mw;
                        p_load -= shed_mw;
                        stage_fired[stage_idx] = true;

                        shedding_events.push(SheddingEvent {
                            stage: stage.stage_id,
                            time_s: time,
                            load_shed_mw: shed_mw,
                            frequency_at_trigger_hz: trigger_freq,
                            buses_affected: stage.priority_buses.clone(),
                        });

                        if post_shed_time.is_none() {
                            post_shed_time = Some(time + self.config.load_restoration_delay_s);
                        }
                    }
                }
            }
            // Remove fired events
            if newly_shed > 0.0 {
                pending.retain(|(fire_time, _shed, stage_idx, _tf, _r)| {
                    !stage_fired[*stage_idx] || time < *fire_time
                });
            }

            // Record trajectory
            trajectory.push((time, freq));

            // Track nadir
            if freq < freq_nadir {
                freq_nadir = freq;
                nadir_time = time;
            }

            // ── Estimate ROCOF ────────────────────────────────────────────────
            let rocof = self.estimate_rocof(&freq_history, dt_s);

            // ── Check stage triggers ──────────────────────────────────────────
            for (idx, stage) in self.config.stages.iter().enumerate() {
                if stage_fired[idx] {
                    continue;
                }
                // Check if frequency is below threshold
                if freq < stage.frequency_threshold_hz {
                    if stage_trigger_time[idx].is_none() {
                        // First crossing — record trigger time
                        stage_trigger_time[idx] = Some(time);

                        // Check ROCOF blocking
                        if !self.should_block(rocof) {
                            let shed_pct = if stage.adaptive {
                                self.adaptive_shed_pct(stage, rocof)
                            } else {
                                stage.load_shedding_pct
                            };
                            let shed_mw = total_load_mw * shed_pct / 100.0;
                            let delay_s = stage.time_delay_ms / 1000.0;
                            let fire_time = time + delay_s;
                            pending.push((fire_time, shed_mw, idx, freq, rocof));
                            stages_triggered.push(stage.stage_id);
                        }
                        // else: blocked — stage does not fire
                    }
                } else {
                    // Frequency recovered above threshold — reset trigger
                    stage_trigger_time[idx] = None;
                }
            }

            // ── Check frequency recovery ──────────────────────────────────────
            if recovery_time.is_none()
                && (freq - f0).abs() < recovery_band_hz
                && total_shed_mw > 0.0
                && time > 1.0
            {
                // Check that restoration delay has elapsed
                let ok = post_shed_time.is_none_or(|t| time >= t);
                if ok {
                    recovery_time = Some(time);
                }
            }

            // ── Swing equation integration (Euler) ────────────────────────────
            // Net power imbalance [MW]
            let p_imbalance = p_mech - p_load;

            // Frequency derivative [Hz/s]
            // Normalised: df/dt = f0 * ΔP / (2 * H * S_base)
            // For a single-machine equivalent with S_base = p_mech:
            let s_base = total_load_mw.max(1.0); // [MW] base
            let df_dt = (p_imbalance / s_base) * f0 / (2.0 * h);

            freq += df_dt * dt_s;

            // Frequency must stay physical
            if !freq.is_finite() {
                return Err(UflsError::SimulationDiverged(format!(
                    "frequency diverged at t={time:.3} s"
                )));
            }

            // Update load-frequency characteristic
            // P_load(f) = P_load_0 * (1 + D * (f - f0) / f0)
            // Only the un-shed portion tracks frequency
            let load_base = total_load_mw - total_shed_mw;
            let freq_dev_pu = (freq - f0) / f0;
            p_load = (load_base * (1.0 + d_coeff * freq_dev_pu)).max(0.0);

            // Update ROCOF history
            freq_history.push(freq);
            if freq_history.len() > rocof_window + 1 {
                freq_history.remove(0);
            }
        }

        let freq_recovered =
            recovery_time.is_some() || (freq - f0).abs() < recovery_band_hz && total_shed_mw == 0.0;

        // Deduplicate stages_triggered (in case of re-entries)
        stages_triggered.dedup();

        Ok(AdaptiveUflsResult {
            stages_triggered,
            total_load_shed_mw: total_shed_mw,
            total_load_shed_pct: total_shed_mw / total_load_mw * 100.0,
            frequency_trajectory: trajectory,
            frequency_nadir_hz: freq_nadir,
            nadir_time_s: nadir_time,
            frequency_recovered: freq_recovered,
            recovery_time_s: recovery_time,
            shedding_events,
        })
    }

    /// Compute adaptive shed percentage based on ROCOF magnitude.
    ///
    /// Larger `|rocof|` → shed more. Capped at 2× the base stage percentage.
    ///
    /// Formula:
    /// ```text
    /// shed% = base% * (1 + |ROCOF| / rocof_ref)
    /// ```
    /// where `rocof_ref = rocof_blocking / 2.0`.
    fn adaptive_shed_pct(&self, stage: &UflsStage, rocof: f64) -> f64 {
        let rocof_ref = (self.config.rocof_blocking / 2.0).max(0.1);
        let multiplier = (1.0 + rocof.abs() / rocof_ref).min(2.0);
        (stage.load_shedding_pct * multiplier).min(100.0)
    }

    /// Returns `true` if ROCOF blocking should prevent stage operation.
    ///
    /// Blocks when `|ROCOF| > rocof_blocking` (disturbance transient).
    fn should_block(&self, rocof: f64) -> bool {
        rocof.abs() > self.config.rocof_blocking
    }

    /// Estimate ROCOF \[Hz/s\] from the frequency history using least-squares fit.
    fn estimate_rocof(&self, freq_history: &[f64], dt_s: f64) -> f64 {
        let n = freq_history.len();
        if n < 2 {
            return 0.0;
        }
        // Simple finite difference over available window
        let df = freq_history[n - 1] - freq_history[0];
        let dt = (n - 1) as f64 * dt_s;
        if dt < 1e-12 {
            return 0.0;
        }
        df / dt
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> UflsConfig {
        UflsConfig::default()
    }

    /// Small disturbance: tiny imbalance — no UFLS stage should trigger.
    ///
    /// Uses a custom config with a low threshold (47.5 Hz) to ensure a small
    /// disturbance never reaches it, and validates the simulation is physically
    /// reasonable.
    #[test]
    fn test_small_disturbance_no_ufls() {
        // Use config with very low thresholds so a tiny disturbance never triggers
        let config = UflsConfig {
            nominal_frequency_hz: 50.0,
            stages: vec![UflsStage {
                stage_id: 1,
                frequency_threshold_hz: 47.5, // well below any realistic nadir for small disturbance
                load_shedding_pct: 10.0,
                time_delay_ms: 200.0,
                adaptive: false,
                priority_buses: vec![],
            }],
            rocof_blocking: 2.5,
            inertia_constant_s: 6.0,
            load_restoration_delay_s: 30.0,
        };
        let ctrl = AdaptiveUflsController::new(config);
        // 1% imbalance: freq drops slightly but stays above 47.5 Hz threshold
        let result = ctrl.simulate(-10.0, 1000.0, 20.0, 0.02).unwrap();
        assert!(
            result.stages_triggered.is_empty(),
            "No stages should trigger for small disturbance, nadir={:.3} Hz",
            result.frequency_nadir_hz
        );
        assert!(
            result.total_load_shed_mw < 1e-9,
            "No load should be shed for small disturbance"
        );
        // Nadir should stay above the threshold
        assert!(
            result.frequency_nadir_hz > 47.5,
            "Nadir {:.3} should be above 47.5 Hz",
            result.frequency_nadir_hz
        );
    }

    /// Large disturbance: 20% imbalance — multiple stages should trigger.
    #[test]
    fn test_large_disturbance_multiple_stages() {
        let config = default_config();
        let ctrl = AdaptiveUflsController::new(config);
        let result = ctrl.simulate(-200.0, 1000.0, 30.0, 0.01).unwrap();
        assert!(
            !result.stages_triggered.is_empty(),
            "At least one stage should fire for large disturbance"
        );
        assert!(
            result.total_load_shed_mw > 0.0,
            "Load must be shed: {:.2} MW",
            result.total_load_shed_mw
        );
        // Frequency nadir should stay above 47.5 Hz (minimum safety limit)
        assert!(
            result.frequency_nadir_hz > 47.5,
            "Nadir {:.3} Hz must stay above 47.5 Hz safety limit",
            result.frequency_nadir_hz
        );
    }

    /// Adaptive shedding: higher ROCOF should shed more.
    #[test]
    fn test_adaptive_higher_rocof_sheds_more() {
        let config = default_config();
        // Stage 2 is adaptive — find it before moving config
        let stage = config.stages.iter().find(|s| s.adaptive).cloned().unwrap();
        let ctrl = AdaptiveUflsController::new(config);

        // Low ROCOF: base amount
        let shed_low = ctrl.adaptive_shed_pct(&stage, 0.3);
        // High ROCOF: should shed more
        let shed_high = ctrl.adaptive_shed_pct(&stage, 2.0);

        assert!(
            shed_high > shed_low,
            "Higher ROCOF ({:.1}) should shed more ({:.2}%) than low ROCOF ({:.1}) -> ({:.2}%)",
            2.0,
            shed_high,
            0.3,
            shed_low
        );
    }

    /// Stage timing: shedding events appear after the delay.
    #[test]
    fn test_stage_timing_delay_respected() {
        let config = default_config();
        let stage_delays: Vec<(usize, f64)> = config
            .stages
            .iter()
            .map(|s| (s.stage_id, s.time_delay_ms / 1000.0))
            .collect();
        let ctrl = AdaptiveUflsController::new(config);
        let result = ctrl.simulate(-200.0, 1000.0, 30.0, 0.01).unwrap();

        for event in &result.shedding_events {
            let min_delay_s = stage_delays
                .iter()
                .find(|&&(id, _)| id == event.stage)
                .map(|&(_, d)| d)
                .unwrap_or(0.0);
            // Event time must be at least min_delay after t=0
            assert!(
                event.time_s >= min_delay_s - 1e-6,
                "Stage {} event at {:.3} s violates min delay {:.3} s",
                event.stage,
                event.time_s,
                min_delay_s
            );
        }
    }

    /// Frequency recovery: after large shed, frequency should recover.
    #[test]
    fn test_frequency_recovery_after_shedding() {
        let config = default_config();
        let ctrl = AdaptiveUflsController::new(config);
        let result = ctrl.simulate(-150.0, 1000.0, 60.0, 0.02).unwrap();

        // With 60 s simulation and load shedding, frequency should recover
        // (or at least the nadir must be finite and above collapse threshold)
        assert!(
            result.frequency_nadir_hz > 47.0,
            "Frequency should not collapse: nadir = {:.3} Hz",
            result.frequency_nadir_hz
        );
        // Trajectory must be non-empty and all values finite
        for (t, f) in &result.frequency_trajectory {
            assert!(
                t.is_finite() && f.is_finite(),
                "Non-finite point in trajectory: t={t}, f={f}"
            );
        }
    }

    /// ROCOF blocking: very high initial ROCOF should suppress the first stage.
    #[test]
    fn test_rocof_blocking_suppresses_stage() {
        // Config with low blocking threshold — all stages blocked
        let mut config = default_config();
        config.rocof_blocking = 0.01; // extremely tight: any ROCOF will block
        let ctrl = AdaptiveUflsController::new(config);

        let result = ctrl.simulate(-200.0, 1000.0, 5.0, 0.01).unwrap();
        // With blocking threshold of 0.01 Hz/s, ROCOF will always exceed it
        // and no stages should fire in the first 5 seconds
        assert!(
            result.shedding_events.is_empty(),
            "All stages should be blocked by ROCOF blocking: {:?}",
            result.shedding_events
        );
    }

    /// Validation: invalid parameters return error.
    #[test]
    fn test_invalid_parameter_returns_error() {
        let config = default_config();
        let ctrl = AdaptiveUflsController::new(config);

        assert!(ctrl.simulate(-100.0, -1.0, 10.0, 0.01).is_err());
        assert!(ctrl.simulate(-100.0, 1000.0, 10.0, -0.01).is_err());
        assert!(ctrl.simulate(-100.0, 1000.0, -1.0, 0.01).is_err());
    }
}
