//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use std::collections::HashMap;

use super::functions::{parse_iso_date, DEFAULT_ENS_MWH, NOW_UNIX, SECS_PER_YEAR, VOLL};
use super::types_3::{
    AssetCategory, AssetCondition, AssetMaintenanceRecord, AssetNameplate, AssetRegistry,
    DefectSeverity, DigitalAssetRecord, MaintenancePlan, TwinSyncStatus,
};

/// Summary of fleet aging.
#[derive(Debug, Clone)]
pub struct AgingReport {
    pub total_assets: usize,
    pub beyond_half_life: usize,
    pub beyond_design_life: usize,
    pub avg_age_years: f64,
    pub oldest_asset_id: String,
    pub oldest_asset_age_years: f64,
    /// Estimated capital required for replacements within 5 years [M€].
    pub capital_replacement_5yr_million_eur: f64,
}
/// Asset health risk level.
#[derive(Clone, Debug, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}
/// Geographic / installation location of a physical asset.
#[derive(Debug, Clone)]
pub struct AssetLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub bay: String,
    pub panel: String,
    pub rack: Option<String>,
}
/// A single asset replacement recommendation from [`MaintenanceScheduler::replacement_prioritization`].
#[derive(Debug, Clone)]
pub struct ReplacementRecommendation {
    /// Asset identifier.
    pub asset_id: String,
    /// Explanation of why replacement is recommended.
    pub reason: String,
    /// Urgency qualifier: `"Immediate"` / `"Within 1yr"` / `"Within 5yr"`.
    pub urgency: String,
    /// Estimated replacement cost \[USD\].
    pub replacement_cost_usd: f64,
    /// Return-on-investment score used for ranking (higher = replace sooner).
    pub roi_score: f64,
}
/// A single scheduled maintenance action within a [`MaintenancePlan`].
#[derive(Debug, Clone)]
pub struct ScheduledMaintenance {
    /// Asset targeted by this maintenance action.
    pub asset_id: String,
    /// Type of maintenance to be performed.
    pub maintenance_type: MaintenanceType,
    /// Calendar year in which maintenance is planned (1-indexed from current year).
    pub scheduled_year: usize,
    /// Estimated total cost of the maintenance activity \[USD\].
    pub estimated_cost_usd: f64,
    /// Risk score before the maintenance intervention.
    pub risk_before: f64,
    /// Estimated residual risk score after the maintenance intervention.
    pub risk_after: f64,
}
/// Category of maintenance work performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceType {
    /// Repair after an observed failure or defect.
    Corrective,
    /// Scheduled time-based servicing.
    Preventive,
    /// Condition-based maintenance triggered by monitoring data.
    Predictive,
    /// Unplanned response to an emergency.
    Emergency,
    /// Major refurbishment restoring near-new condition.
    Overhaul,
}
/// Live telemetry snapshot for an asset.
#[derive(Debug, Clone)]
pub struct AssetTelemetry {
    /// Unix timestamp `s`.
    pub timestamp: f64,
    pub current_ka: f64,
    pub voltage_kv: f64,
    pub power_mw: f64,
    pub temperature_c: f64,
    /// For rotating machines [mm/s].
    pub vibration_mm_per_s: f64,
    /// HV equipment partial discharge `pC`.
    pub partial_discharge_pc: f64,
    /// Transformer oil temperature [°C].
    pub oil_temperature_c: f64,
    /// H₂ dissolved in oil — DGA indicator `ppm`.
    pub dissolved_gas_h2_ppm: f64,
    /// GIS equipment SF₆ pressure `bar`.
    pub sf6_pressure_bar: f64,
}
/// Category of maintenance work.
#[derive(Debug, Clone)]
pub enum AssetMaintenanceType {
    Routine,
    Preventive,
    Corrective,
    Emergency,
    Overhaul,
}
/// Record of a single asset failure event.
#[derive(Debug, Clone)]
pub struct FailureRecord {
    pub date: String,
    pub failure_mode: String,
    pub severity: RiskLevel,
    pub outage_duration_hours: f64,
    pub affected_customers: u32,
    pub repair_cost_eur: f64,
    pub root_cause: String,
    pub corrective_action: String,
}
/// Classification of a power grid asset, carrying class-specific nameplate data.
#[derive(Debug, Clone)]
pub enum AssetClass {
    /// Oil-filled or dry-type power transformer.
    PowerTransformer {
        /// Rated apparent power \[MVA\].
        mva_rating: f64,
        /// Nominal voltage (highest winding) \[kV\].
        voltage_kv: f64,
        /// IEC vector group (e.g. `"Dyn11"`).
        vector_group: String,
    },
    /// High-voltage circuit breaker (SF6, vacuum, oil).
    CircuitBreaker {
        /// Rated voltage \[kV\].
        rated_kv: f64,
        /// Rated normal current \[kA\].
        rated_ka: f64,
        /// Short-circuit interrupting capacity \[kA\].
        interrupting_capacity_ka: f64,
    },
    /// Overhead or underground transmission/distribution line.
    TransmissionLine {
        /// Operating voltage \[kV\].
        voltage_kv: f64,
        /// Route length \[km\].
        length_km: f64,
        /// Conductor type (e.g. `"ACSR 400mm²"`).
        conductor_type: String,
    },
    /// Tower, pole, or other overhead line support structure.
    OverheadLineStructure {
        /// Structure material / design class (e.g. `"Lattice Steel"`).
        structure_type: String,
        /// Nominal height above ground \[m\].
        height_m: f64,
    },
    /// Instrument transformer (potential or current).
    MeasuringTransformer {
        /// `"PT"` for voltage / potential transformer, `"CT"` for current transformer.
        pt_or_ct: String,
        /// Rated primary voltage \[kV\].
        rated_kv: f64,
    },
    /// Numerical or electromechanical protection relay.
    ProtectionRelay {
        /// Relay manufacturer.
        make: String,
        /// Relay model designation.
        model: String,
        /// Communication protocol (e.g. `"IEC 61850"`, `"DNP3"`).
        protocol: String,
    },
    /// Stationary battery bank (BESS or UPS duty).
    BatteryBank {
        /// Nominal energy capacity \[kWh\].
        capacity_kwh: f64,
        /// Electrochemical chemistry (e.g. `"VRLA"`, `"Li-NMC"`).
        chemistry: String,
        /// Number of cells in series per string.
        cells_in_series: u32,
    },
}
impl AssetClass {
    /// Characteristic life η used in the Weibull failure model \[years\].
    pub fn characteristic_life_years(&self) -> f64 {
        match self {
            AssetClass::PowerTransformer { .. } => 40.0,
            AssetClass::CircuitBreaker { .. } => 30.0,
            AssetClass::TransmissionLine { .. } => 50.0,
            AssetClass::OverheadLineStructure { .. } => 60.0,
            AssetClass::MeasuringTransformer { .. } => 35.0,
            AssetClass::ProtectionRelay { .. } => 25.0,
            AssetClass::BatteryBank { .. } => 15.0,
        }
    }
    /// Typical replacement cost \[USD\].
    pub fn replacement_cost_usd(&self) -> f64 {
        match self {
            AssetClass::PowerTransformer { .. } => 500_000.0,
            AssetClass::CircuitBreaker { .. } => 50_000.0,
            AssetClass::TransmissionLine { .. } => 200_000.0,
            AssetClass::OverheadLineStructure { .. } => 20_000.0,
            AssetClass::MeasuringTransformer { .. } => 15_000.0,
            AssetClass::ProtectionRelay { .. } => 10_000.0,
            AssetClass::BatteryBank { .. } => 100_000.0,
        }
    }
    /// Estimated preventive maintenance cost per event \[USD\].
    pub fn maintenance_cost_usd(&self) -> f64 {
        match self {
            AssetClass::PowerTransformer { .. } => 20_000.0,
            AssetClass::CircuitBreaker { .. } => 5_000.0,
            AssetClass::TransmissionLine { .. } => 15_000.0,
            AssetClass::OverheadLineStructure { .. } => 2_000.0,
            AssetClass::MeasuringTransformer { .. } => 1_500.0,
            AssetClass::ProtectionRelay { .. } => 1_000.0,
            AssetClass::BatteryBank { .. } => 8_000.0,
        }
    }
}
/// Record of a maintenance intervention on a grid asset.
#[derive(Debug, Clone)]
pub struct MaintenanceRecord {
    /// Maintenance date as Unix epoch \[s\].
    pub date_unix: f64,
    /// Category of maintenance performed.
    pub maintenance_type: MaintenanceType,
    /// Total cost of the maintenance activity \[USD\].
    pub cost_usd: f64,
    /// Duration of asset outage associated with maintenance \[h\].
    pub downtime_h: f64,
    /// Organisation or individual who performed the work.
    pub performed_by: String,
    /// List of material parts or components replaced.
    pub parts_replaced: Vec<String>,
}
/// Output of [`MaintenanceScheduler::dga_analysis`] (Dissolved Gas Analysis).
#[derive(Debug, Clone)]
pub struct DgaResult {
    /// Human-readable fault type classification.
    pub fault_type: String,
    /// Severity of the identified fault condition.
    pub severity: DefectSeverity,
    /// Rogers Ratios: (CH₄/H₂, C₂H₂/C₂H₄, C₂H₄/C₂H₆).
    pub rogers_ratios: (f64, f64, f64),
    /// Recommended corrective action.
    pub recommended_action: String,
}
/// Engine for monitoring and updating digital twin synchronisation.
pub struct TwinSynchronizer {
    /// Seconds after which an asset twin is considered stale.
    pub stale_threshold_s: f64,
    /// Minimum acceptable accuracy score (0–1).
    pub min_accuracy_threshold: f64,
}
impl TwinSynchronizer {
    /// Create a new synchroniser with the given thresholds.
    pub fn new(stale_threshold_s: f64, min_accuracy_threshold: f64) -> Self {
        Self {
            stale_threshold_s,
            min_accuracy_threshold,
        }
    }
    /// Check synchronisation status of every asset in the registry.
    pub fn check_sync_status(
        &self,
        registry: &AssetRegistry,
        current_time: f64,
    ) -> Vec<TwinSyncStatus> {
        registry
            .assets
            .iter()
            .map(|asset| {
                let last_sync_time = asset.telemetry.timestamp;
                let sync_gap_s = (current_time - last_sync_time).max(0.0);
                let is_stale = sync_gap_s > self.stale_threshold_s;
                let mut missing = Vec::new();
                if asset.telemetry.current_ka == 0.0 {
                    missing.push("current_ka".to_string());
                }
                if asset.telemetry.voltage_kv == 0.0 {
                    missing.push("voltage_kv".to_string());
                }
                if asset.telemetry.power_mw == 0.0 {
                    missing.push("power_mw".to_string());
                }
                if (asset.telemetry.temperature_c == 0.0 || asset.telemetry.temperature_c == 20.0)
                    && last_sync_time == 0.0
                {
                    missing.push("temperature_c".to_string());
                }
                let total_sensors = 7usize;
                let missing_count = missing.len().min(total_sensors);
                let data_quality_pct =
                    ((total_sensors - missing_count) as f64 / total_sensors as f64) * 100.0;
                TwinSyncStatus {
                    asset_id: asset.asset_id.clone(),
                    last_sync_time,
                    sync_gap_s,
                    data_quality_pct,
                    missing_sensors: missing,
                    is_stale,
                    accuracy: asset.digital_twin_accuracy,
                }
            })
            .collect()
    }
    /// Update the telemetry for the named asset in the registry.
    ///
    /// Returns `OxiGridError::InvalidParameter` if the asset is not found.
    pub fn update_telemetry(
        registry: &mut AssetRegistry,
        asset_id: &str,
        telemetry: AssetTelemetry,
    ) -> Result<(), crate::error::OxiGridError> {
        match registry.get_asset_mut(asset_id) {
            Some(asset) => {
                asset.telemetry = telemetry;
                Ok(())
            }
            None => Err(crate::error::OxiGridError::InvalidParameter(format!(
                "asset '{}' not found in registry",
                asset_id
            ))),
        }
    }
    /// Compute a digital twin accuracy score for an asset.
    ///
    /// Starts at 1.0 and deducts:
    /// - `−0.05` per missing telemetry field (zero-value proxy)
    /// - `−0.1` if no maintenance record in the last year (based on empty history)
    /// - `−0.02` per active defect code
    pub fn compute_accuracy(asset: &AssetDigitalTwin) -> f64 {
        let mut score = 1.0_f64;
        if asset.telemetry.timestamp == 0.0 {
            score -= 0.05;
        }
        if asset.telemetry.current_ka == 0.0 {
            score -= 0.05;
        }
        if asset.telemetry.voltage_kv == 0.0 {
            score -= 0.05;
        }
        if asset.telemetry.power_mw == 0.0 {
            score -= 0.05;
        }
        if asset.telemetry.vibration_mm_per_s == 0.0 {
            score -= 0.05;
        }
        if asset.telemetry.partial_discharge_pc == 0.0 {
            score -= 0.05;
        }
        if asset.telemetry.dissolved_gas_h2_ppm == 0.0 {
            score -= 0.05;
        }
        let one_year_s = SECS_PER_YEAR;
        let recent_maintenance = asset.maintenance_history.iter().any(|m| {
            parse_iso_date(&m.date)
                .map(|t| (NOW_UNIX - t) < one_year_s)
                .unwrap_or(false)
        });
        if !recent_maintenance {
            score -= 0.1;
        }
        let defect_penalty = 0.02 * asset.condition.defect_codes.len() as f64;
        score -= defect_penalty;
        score.clamp(0.0, 1.0)
    }
}
/// Output of [`MaintenanceScheduler::asset_health_index`].
#[derive(Debug, Clone)]
pub struct AssetHealthReport {
    /// Asset identifier.
    pub asset_id: String,
    /// Composite health index (0 = end-of-life, 100 = new/excellent).
    pub health_index: f64,
    /// Qualitative condition band: `"Good"` / `"Fair"` / `"Poor"` / `"Critical"`.
    pub condition_band: String,
    /// Estimated remaining serviceable life \[years\].
    pub remaining_life_years: f64,
    /// `true` if the asset requires priority maintenance attention.
    pub priority_maintenance: bool,
}
/// Condition-based maintenance assessor using rule-based telemetry analysis.
pub struct CbmAssessor;
impl CbmAssessor {
    /// Assess risk level from telemetry (rule-based).
    ///
    /// Rules:
    /// - Vibration > 7.1 mm/s (ISO 10816 Zone D) → `Critical`
    /// - H₂ in oil > 150 ppm → `Critical`
    /// - H₂ in oil > 50 ppm → `High`
    /// - PD > 1000 pC → `High`
    /// - Temperature > 120 °C → `High`
    /// - Temperature > 100 °C → `Medium`
    /// - Otherwise → `Low`
    pub fn assess_from_telemetry(telemetry: &AssetTelemetry) -> RiskLevel {
        if telemetry.vibration_mm_per_s > 7.1 {
            return RiskLevel::Critical;
        }
        if telemetry.dissolved_gas_h2_ppm > 150.0 {
            return RiskLevel::Critical;
        }
        if telemetry.dissolved_gas_h2_ppm > 50.0 {
            return RiskLevel::High;
        }
        if telemetry.partial_discharge_pc > 1000.0 {
            return RiskLevel::High;
        }
        if telemetry.temperature_c > 120.0 {
            return RiskLevel::High;
        }
        if telemetry.temperature_c > 100.0 {
            return RiskLevel::Medium;
        }
        RiskLevel::Low
    }
    /// Update the asset's condition risk level based on its current telemetry.
    pub fn update_condition(asset: &mut AssetDigitalTwin) {
        let risk = Self::assess_from_telemetry(&asset.telemetry);
        asset.condition.risk_level = risk;
    }
    /// Estimate remaining useful life `years`.
    ///
    /// Uses linear degradation from health index:
    /// `RUL = (health - threshold_health) / (100 - threshold_health) * typical_life_years`
    ///
    /// Returns 0.0 when health is at or below threshold.
    pub fn estimate_rul(
        asset: &AssetDigitalTwin,
        threshold_health: f64,
        typical_life_years: f64,
    ) -> f64 {
        let health = asset.condition.overall_health_index;
        let span = 100.0 - threshold_health;
        if span <= 0.0 || health <= threshold_health {
            return 0.0;
        }
        let rul = ((health - threshold_health) / span) * typical_life_years;
        rul.max(0.0)
    }
    /// Recommend a maintenance interval in years using a simplified CBM criterion.
    ///
    /// Based on current health index:
    /// - Health ≥ 80 → 5 years
    /// - Health ≥ 60 → 3 years
    /// - Health ≥ 40 → 1 year
    /// - Health < 40  → 0.5 years (6 months)
    pub fn recommend_maintenance_interval(asset: &AssetDigitalTwin) -> f64 {
        let health = asset.condition.overall_health_index;
        if health >= 80.0 {
            5.0
        } else if health >= 60.0 {
            3.0
        } else if health >= 40.0 {
            1.0
        } else {
            0.5
        }
    }
}
/// Category of inspection performed on a grid asset.
#[derive(Debug, Clone)]
pub enum InspectionType {
    /// Unaided eye survey of physical condition.
    Visual,
    /// Infrared thermographic survey.
    Thermal {
        /// Whether a calibrated thermal camera was used (vs. spot pyrometer).
        camera_used: bool,
    },
    /// Dissolved Gas Analysis of transformer insulating oil.
    DGA,
    /// Partial Discharge detection (acoustic, UHF, or electrical).
    PartialDischarge,
    /// Insulation Resistance / Polarisation Index test.
    InsulationResistance,
    /// Contact Resistance measurement (μΩ).
    ContactResistance,
    /// Vibration / acoustic signature analysis.
    Vibration,
    /// Ultrasonic DGA (online, non-intrusive).
    UltrasonicDGA,
}
/// Fleet-level maintenance planning engine.
///
/// Holds a portfolio of [`DigitalAssetRecord`]s and exposes methods for
/// condition assessment, failure prediction, risk scoring, and capital planning.
#[derive(Debug, Clone)]
pub struct MaintenanceScheduler {
    /// Portfolio of assets under management.
    pub assets: Vec<DigitalAssetRecord>,
    /// Annual capital / O&M budget available for maintenance \[USD/yr\].
    pub budget_annual_usd: f64,
    /// Number of years over which to plan maintenance activities \[years\].
    pub planning_horizon_years: usize,
}
impl MaintenanceScheduler {
    /// Create a new scheduler with the given asset portfolio and financial parameters.
    pub fn new(
        assets: Vec<DigitalAssetRecord>,
        budget_annual_usd: f64,
        planning_horizon_years: usize,
    ) -> Self {
        Self {
            assets,
            budget_annual_usd,
            planning_horizon_years,
        }
    }
    /// Return the current condition score for `asset` \[0–5\].
    ///
    /// Starts from the most recent inspection score and applies a linear
    /// degradation of 0.1 per year since that inspection.  Returns a default
    /// of 3.0 if no inspections exist.
    pub fn current_condition_score(&self, asset: &DigitalAssetRecord) -> f64 {
        let latest = asset.inspection_history.iter().max_by(|a, b| {
            a.date_unix
                .partial_cmp(&b.date_unix)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        match latest {
            None => 3.0,
            Some(rec) => {
                let years_since = (NOW_UNIX - rec.date_unix) / SECS_PER_YEAR;
                let degraded = rec.condition_score - 0.1 * years_since.max(0.0);
                degraded.clamp(0.0, 5.0)
            }
        }
    }
    /// Estimate the probability of failure within `horizon_years` using a
    /// two-parameter Weibull model P(T < t) = 1 − exp(−(t/η)^β).
    ///
    /// β = 2.5 (typical for electrical HV equipment).
    /// η is the class-specific characteristic life, reduced by 20 % when the
    /// current condition score is below 3.0.
    pub fn predict_failure_probability(
        &self,
        asset: &DigitalAssetRecord,
        horizon_years: f64,
    ) -> f64 {
        let beta: f64 = 2.5;
        let mut eta = asset.asset_class.characteristic_life_years();
        let age = (NOW_UNIX - asset.installation_date_unix) / SECS_PER_YEAR;
        let condition = self.current_condition_score(asset);
        if condition < 3.0 {
            eta *= 0.8;
        }
        let t = (age + horizon_years).max(0.0);
        let p: f64 = 1.0 - (-(t / eta).powf(beta)).exp();
        p.clamp(0.0, 1.0)
    }
    /// Compute a dimensionless risk score for `asset`.
    ///
    /// risk = P(failure in 1 yr) × criticality × consequence \[MUSD\]
    pub fn calculate_risk_score(&self, asset: &DigitalAssetRecord) -> f64 {
        let failure_prob = self.predict_failure_probability(asset, 1.0);
        let consequence_usd = asset.asset_class.replacement_cost_usd() + DEFAULT_ENS_MWH * VOLL;
        failure_prob * asset.criticality_score * consequence_usd / 1e6
    }
    /// Build a risk-prioritised maintenance plan constrained by the available
    /// budget over `planning_horizon_years`.
    ///
    /// Assets with a risk score above `risk_threshold` are candidates.  They
    /// are sorted by risk (highest first) and scheduled until the budget is
    /// exhausted.  The risk is assumed to fall by 70 % after each maintenance
    /// event.
    pub fn generate_maintenance_plan(&self, risk_threshold: f64) -> MaintenancePlan {
        let total_budget = self.budget_annual_usd * self.planning_horizon_years as f64;
        let mut candidates: Vec<(&DigitalAssetRecord, f64)> = self
            .assets
            .iter()
            .map(|a| (a, self.calculate_risk_score(a)))
            .filter(|(_, r)| *r > risk_threshold)
            .collect();
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let total_risk_all: f64 = self
            .assets
            .iter()
            .map(|a| self.calculate_risk_score(a))
            .sum();
        let mut scheduled_actions: Vec<ScheduledMaintenance> = Vec::new();
        let mut total_cost_usd = 0.0;
        let mut total_risk_before = 0.0;
        let mut total_risk_after = 0.0;
        let mut assets_not_covered: Vec<String> = Vec::new();
        let mut remaining_budget = total_budget;
        for (asset, risk_before) in &candidates {
            let cost = asset.asset_class.maintenance_cost_usd();
            if cost <= remaining_budget {
                let risk_after = risk_before * 0.3;
                let year_slot = scheduled_actions.len() % self.planning_horizon_years.max(1) + 1;
                scheduled_actions.push(ScheduledMaintenance {
                    asset_id: asset.asset_id.clone(),
                    maintenance_type: MaintenanceType::Predictive,
                    scheduled_year: year_slot,
                    estimated_cost_usd: cost,
                    risk_before: *risk_before,
                    risk_after,
                });
                total_cost_usd += cost;
                total_risk_before += risk_before;
                total_risk_after += risk_after;
                remaining_budget -= cost;
            } else {
                assets_not_covered.push(asset.asset_id.clone());
            }
        }
        let risk_reduction_pct = if total_risk_all > 0.0 {
            (total_risk_before - total_risk_after) / total_risk_all * 100.0
        } else {
            0.0
        };
        MaintenancePlan {
            scheduled_actions,
            total_cost_usd,
            risk_reduction_pct,
            assets_not_covered,
        }
    }
    /// Diagnose transformer oil health using the Rogers Ratio method.
    ///
    /// `gas_ppm` must contain keys `"CH4"`, `"C2H2"`, `"C2H4"`, `"C2H6"`, `"H2"`.
    /// Returns `Err` if the asset is not a `PowerTransformer` or if a required
    /// gas key is missing.
    pub fn dga_analysis(
        &self,
        gas_ppm: &HashMap<String, f64>,
        asset: &DigitalAssetRecord,
    ) -> Result<DgaResult, String> {
        match &asset.asset_class {
            AssetClass::PowerTransformer { .. } => {}
            _ => {
                return Err(format!(
                    "DGA analysis only applicable to PowerTransformer, asset {} is {:?}",
                    asset.asset_id,
                    std::mem::discriminant(&asset.asset_class)
                ));
            }
        }
        let get = |key: &str| -> Result<f64, String> {
            gas_ppm
                .get(key)
                .copied()
                .ok_or_else(|| format!("Missing gas key '{}' in gas_ppm map", key))
        };
        let ch4 = get("CH4")?;
        let c2h2 = get("C2H2")?;
        let c2h4 = get("C2H4")?;
        let c2h6 = get("C2H6")?;
        let h2 = get("H2")?;
        let r1 = if h2 > 0.0 { ch4 / h2 } else { 0.0 };
        let r2 = if c2h4 > 0.0 { c2h2 / c2h4 } else { 0.0 };
        let r3 = if c2h6 > 0.0 { c2h4 / c2h6 } else { 0.0 };
        let (fault_type, severity, recommended_action) = if r2 > 1.0 && c2h2 > 5.0 {
            (
                "Electrical Discharge (Arcing)".to_string(),
                DefectSeverity::Critical,
                "Immediate de-energisation and internal inspection required.".to_string(),
            )
        } else if r2 > 0.1 && c2h2 > 1.0 {
            (
                "Low-Energy Electrical Discharge".to_string(),
                DefectSeverity::Significant,
                "Schedule urgent internal inspection within 30 days.".to_string(),
            )
        } else if r3 > 4.0 && r1 > 1.0 {
            (
                "High-Temperature Thermal Fault (>700°C)".to_string(),
                DefectSeverity::Critical,
                "Reduce loading immediately; schedule emergency inspection.".to_string(),
            )
        } else if r3 > 1.0 && r1 > 0.5 {
            (
                "Medium-Temperature Thermal Fault (300-700°C)".to_string(),
                DefectSeverity::Moderate,
                "Increase DGA sampling frequency; plan maintenance outage.".to_string(),
            )
        } else if r3 <= 1.0 && r1 < 0.5 {
            (
                "Normal Aging / Low Temperature Thermal".to_string(),
                DefectSeverity::Minor,
                "Continue routine monitoring; next DGA within 12 months.".to_string(),
            )
        } else {
            (
                "Normal".to_string(),
                DefectSeverity::None,
                "No action required; maintain standard monitoring schedule.".to_string(),
            )
        };
        Ok(DgaResult {
            fault_type,
            severity,
            rogers_ratios: (r1, r2, r3),
            recommended_action,
        })
    }
    /// Compute the Asset Health Index (AHI) for `asset` \[0–100\].
    ///
    /// AHI = 100 × (0.4 × condition_norm + 0.3 × age_score + 0.3 × maint_eff)
    ///
    /// where:
    /// - `condition_norm` = current_condition_score / 5
    /// - `age_score` = 1 − age / expected_life (clamped 0–1)
    /// - `maint_eff` = fraction of maintenance actions that are preventive or predictive
    pub fn asset_health_index(&self, asset: &DigitalAssetRecord) -> AssetHealthReport {
        let condition = self.current_condition_score(asset);
        let condition_norm = (condition / 5.0).clamp(0.0, 1.0);
        let expected_life = asset.asset_class.characteristic_life_years();
        let age =
            ((NOW_UNIX - asset.installation_date_unix) / SECS_PER_YEAR).clamp(0.0, expected_life);
        let age_score: f64 = (1.0 - age / expected_life).clamp(0.0, 1.0);
        let maint_eff = if asset.maintenance_history.is_empty() {
            0.0
        } else {
            let proactive = asset
                .maintenance_history
                .iter()
                .filter(|m| {
                    m.maintenance_type == MaintenanceType::Preventive
                        || m.maintenance_type == MaintenanceType::Predictive
                })
                .count();
            proactive as f64 / asset.maintenance_history.len() as f64
        };
        let ahi = (0.4 * condition_norm + 0.3 * age_score + 0.3 * maint_eff) * 100.0;
        let ahi = ahi.clamp(0.0, 100.0);
        let condition_band = if ahi >= 75.0 {
            "Good".to_string()
        } else if ahi >= 50.0 {
            "Fair".to_string()
        } else if ahi >= 25.0 {
            "Poor".to_string()
        } else {
            "Critical".to_string()
        };
        let remaining_life_years = (expected_life - age).clamp(0.0, expected_life);
        let priority_maintenance = ahi < 50.0;
        AssetHealthReport {
            asset_id: asset.asset_id.clone(),
            health_index: ahi,
            condition_band,
            remaining_life_years,
            priority_maintenance,
        }
    }
    /// Identify assets that should be replaced and rank them by ROI score
    /// within the given `budget` \[USD\].
    ///
    /// ROI score = (failure_prob × criticality) / (replacement_cost / 1e6)
    pub fn replacement_prioritization(&self, budget: f64) -> Vec<ReplacementRecommendation> {
        let mut candidates: Vec<(&DigitalAssetRecord, f64, f64, String, String)> = Vec::new();
        for asset in &self.assets {
            let condition = self.current_condition_score(asset);
            let latest_severity = asset
                .inspection_history
                .iter()
                .max_by(|a, b| {
                    a.date_unix
                        .partial_cmp(&b.date_unix)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|r| &r.defect_severity);
            let is_eol = latest_severity == Some(&DefectSeverity::EndOfLife);
            let is_critical_defect = latest_severity == Some(&DefectSeverity::Critical);
            let needs_replacement = condition < 2.0 || is_eol || is_critical_defect;
            if !needs_replacement {
                continue;
            }
            let failure_prob = self.predict_failure_probability(asset, 1.0);
            let replacement_cost = asset.asset_class.replacement_cost_usd();
            let roi_score =
                (failure_prob * asset.criticality_score) / (replacement_cost / 1e6).max(1e-9);
            let urgency = if condition < 1.0 || is_eol {
                "Immediate".to_string()
            } else if condition < 2.0 {
                "Within 1yr".to_string()
            } else {
                "Within 5yr".to_string()
            };
            let reason = if is_eol {
                "Asset has reached end-of-life per latest inspection.".to_string()
            } else if is_critical_defect {
                "Critical defect identified in latest inspection.".to_string()
            } else {
                format!(
                    "Condition score {:.2} below replacement threshold of 2.0.",
                    condition
                )
            };
            candidates.push((asset, roi_score, replacement_cost, reason, urgency));
        }
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut recommendations: Vec<ReplacementRecommendation> = Vec::new();
        let mut remaining_budget = budget;
        for (asset, roi_score, replacement_cost, reason, urgency) in candidates {
            if replacement_cost <= remaining_budget {
                recommendations.push(ReplacementRecommendation {
                    asset_id: asset.asset_id.clone(),
                    reason,
                    urgency,
                    replacement_cost_usd: replacement_cost,
                    roi_score,
                });
                remaining_budget -= replacement_cost;
            }
        }
        recommendations
    }
    /// Compute summary statistics for the entire managed fleet.
    pub fn fleet_statistics(&self) -> FleetReport {
        if self.assets.is_empty() {
            return FleetReport {
                mean_age_years: 0.0,
                median_condition: 0.0,
                condition_distribution: vec![
                    ("0-1".to_string(), 0),
                    ("1-2".to_string(), 0),
                    ("2-3".to_string(), 0),
                    ("3-4".to_string(), 0),
                    ("4-5".to_string(), 0),
                ],
                total_replacement_value_usd: 0.0,
                high_risk_assets: 0,
            };
        }
        let mean_age_years = self
            .assets
            .iter()
            .map(|a| (NOW_UNIX - a.installation_date_unix) / SECS_PER_YEAR)
            .sum::<f64>()
            / self.assets.len() as f64;
        let mut scores: Vec<f64> = self
            .assets
            .iter()
            .map(|a| self.current_condition_score(a))
            .collect();
        scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = scores.len();
        let median_condition = if n.is_multiple_of(2) {
            (scores[n / 2 - 1] + scores[n / 2]) / 2.0
        } else {
            scores[n / 2]
        };
        let bands = ["0-1", "1-2", "2-3", "3-4", "4-5"];
        let mut dist = [0usize; 5];
        for &s in &scores {
            let idx = if s < 1.0 {
                0
            } else if s < 2.0 {
                1
            } else if s < 3.0 {
                2
            } else if s < 4.0 {
                3
            } else {
                4
            };
            dist[idx] += 1;
        }
        let condition_distribution = bands
            .iter()
            .zip(dist.iter())
            .map(|(b, c)| (b.to_string(), *c))
            .collect();
        let total_replacement_value_usd = self
            .assets
            .iter()
            .map(|a| a.asset_class.replacement_cost_usd())
            .sum();
        let high_risk_assets = self
            .assets
            .iter()
            .filter(|a| self.calculate_risk_score(a) > 0.1)
            .count();
        FleetReport {
            mean_age_years,
            median_condition,
            condition_distribution,
            total_replacement_value_usd,
            high_risk_assets,
        }
    }
}
/// Full record of a single asset inspection event.
#[derive(Debug, Clone)]
pub struct InspectionRecord {
    /// Unique identifier for this inspection event.
    pub inspection_id: String,
    /// Inspection date as Unix epoch \[s\].
    pub date_unix: f64,
    /// Type of inspection performed.
    pub inspection_type: InspectionType,
    /// Employee or contractor identifier of the inspector.
    pub inspector_id: String,
    /// Free-text findings noted during inspection.
    pub findings: Vec<String>,
    /// Overall condition score assigned (0.0 = failed, 5.0 = new/excellent).
    pub condition_score: f64,
    /// Recommended corrective or preventive actions.
    pub recommended_actions: Vec<String>,
    /// Highest severity defect identified.
    pub defect_severity: DefectSeverity,
}
/// Summary statistics for the entire asset fleet.
#[derive(Debug, Clone)]
pub struct FleetReport {
    /// Population-mean asset age \[years\].
    pub mean_age_years: f64,
    /// Median current condition score across all assets (0–5 scale).
    pub median_condition: f64,
    /// Count of assets in each 1-point condition band: `"0-1"`, `"1-2"`, …, `"4-5"`.
    pub condition_distribution: Vec<(String, usize)>,
    /// Sum of replacement costs for all assets \[USD\].
    pub total_replacement_value_usd: f64,
    /// Number of assets whose risk score exceeds 0.1.
    pub high_risk_assets: usize,
}
/// Full digital twin of a single physical power grid asset.
#[derive(Debug, Clone)]
pub struct AssetDigitalTwin {
    pub asset_id: String,
    pub asset_name: String,
    pub category: AssetCategory,
    pub substation_id: String,
    /// ISO 8601 date.
    pub commissioning_date: String,
    pub manufacturer: String,
    pub model_number: String,
    pub serial_number: String,
    pub location: AssetLocation,
    pub condition: AssetCondition,
    pub nameplate: AssetNameplate,
    pub telemetry: AssetTelemetry,
    pub maintenance_history: Vec<AssetMaintenanceRecord>,
    pub failure_history: Vec<FailureRecord>,
    /// 0–1: how well the digital twin matches the physical asset.
    pub digital_twin_accuracy: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_class_characteristic_life_bounds() {
        let xfmr = AssetClass::PowerTransformer {
            mva_rating: 100.0,
            voltage_kv: 110.0,
            vector_group: "Dyn11".to_string(),
        };
        let batt = AssetClass::BatteryBank {
            capacity_kwh: 500.0,
            chemistry: "VRLA".to_string(),
            cells_in_series: 24,
        };
        assert!((xfmr.characteristic_life_years() - 40.0).abs() < f64::EPSILON);
        assert!((batt.characteristic_life_years() - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn asset_class_replacement_cost_ordering() {
        let xfmr = AssetClass::PowerTransformer {
            mva_rating: 100.0,
            voltage_kv: 110.0,
            vector_group: "Dyn11".to_string(),
        };
        let batt = AssetClass::BatteryBank {
            capacity_kwh: 500.0,
            chemistry: "VRLA".to_string(),
            cells_in_series: 24,
        };
        let cb = AssetClass::CircuitBreaker {
            rated_kv: 145.0,
            rated_ka: 2.0,
            interrupting_capacity_ka: 40.0,
        };
        assert!(xfmr.replacement_cost_usd() > batt.replacement_cost_usd());
        assert!(batt.replacement_cost_usd() > cb.replacement_cost_usd());
    }

    #[test]
    fn asset_class_maintenance_cost_nonnegative() {
        let variants: Vec<AssetClass> = vec![
            AssetClass::PowerTransformer {
                mva_rating: 50.0,
                voltage_kv: 110.0,
                vector_group: "Dyn11".to_string(),
            },
            AssetClass::CircuitBreaker {
                rated_kv: 145.0,
                rated_ka: 2.0,
                interrupting_capacity_ka: 40.0,
            },
            AssetClass::TransmissionLine {
                voltage_kv: 110.0,
                length_km: 50.0,
                conductor_type: "ACSR".to_string(),
            },
            AssetClass::OverheadLineStructure {
                structure_type: "Lattice".to_string(),
                height_m: 30.0,
            },
            AssetClass::MeasuringTransformer {
                pt_or_ct: "PT".to_string(),
                rated_kv: 110.0,
            },
            AssetClass::ProtectionRelay {
                make: "SEL".to_string(),
                model: "421".to_string(),
                protocol: "IEC 61850".to_string(),
            },
            AssetClass::BatteryBank {
                capacity_kwh: 200.0,
                chemistry: "Li-NMC".to_string(),
                cells_in_series: 48,
            },
        ];
        for v in &variants {
            assert!(
                v.maintenance_cost_usd() >= 0.0,
                "maintenance cost must be non-negative"
            );
        }
    }

    #[test]
    fn risk_level_enum_equality() {
        assert_eq!(RiskLevel::High, RiskLevel::High);
        assert_eq!(RiskLevel::Low, RiskLevel::Low);
        assert_ne!(RiskLevel::Low, RiskLevel::Critical);
        assert_ne!(RiskLevel::Medium, RiskLevel::High);
    }

    #[test]
    fn telemetry_fields_direct_access() {
        let t = AssetTelemetry {
            timestamp: 1_710_000_000.0,
            current_ka: 1.2,
            voltage_kv: 110.5,
            power_mw: 132.0,
            temperature_c: 75.0,
            vibration_mm_per_s: 2.5,
            partial_discharge_pc: 15.0,
            oil_temperature_c: 68.0,
            dissolved_gas_h2_ppm: 25.0,
            sf6_pressure_bar: 5.8,
        };
        assert!((t.timestamp - 1_710_000_000.0).abs() < f64::EPSILON);
        assert!((t.current_ka - 1.2).abs() < 1e-10);
        assert!((t.voltage_kv - 110.5).abs() < 1e-10);
        assert!((t.power_mw - 132.0).abs() < 1e-10);
        assert!((t.temperature_c - 75.0).abs() < 1e-10);
        assert!((t.vibration_mm_per_s - 2.5).abs() < 1e-10);
        assert!((t.partial_discharge_pc - 15.0).abs() < 1e-10);
        assert!((t.oil_temperature_c - 68.0).abs() < 1e-10);
        assert!((t.dissolved_gas_h2_ppm - 25.0).abs() < 1e-10);
        assert!((t.sf6_pressure_bar - 5.8).abs() < 1e-10);
    }

    #[test]
    fn maintenance_type_variants_eq() {
        assert_eq!(MaintenanceType::Predictive, MaintenanceType::Predictive);
        assert_eq!(MaintenanceType::Corrective, MaintenanceType::Corrective);
        assert_ne!(MaintenanceType::Preventive, MaintenanceType::Emergency);
        assert_ne!(MaintenanceType::Overhaul, MaintenanceType::Predictive);
    }

    #[test]
    fn cbm_assessor_vibration_critical() {
        let t = AssetTelemetry {
            timestamp: 0.0,
            current_ka: 1.0,
            voltage_kv: 110.0,
            power_mw: 100.0,
            temperature_c: 60.0,
            vibration_mm_per_s: 8.0,
            partial_discharge_pc: 5.0,
            oil_temperature_c: 55.0,
            dissolved_gas_h2_ppm: 10.0,
            sf6_pressure_bar: 6.0,
        };
        assert_eq!(CbmAssessor::assess_from_telemetry(&t), RiskLevel::Critical);
    }

    #[test]
    fn cbm_assessor_normal_telemetry_low() {
        let t = AssetTelemetry {
            timestamp: 0.0,
            current_ka: 0.5,
            voltage_kv: 110.0,
            power_mw: 50.0,
            temperature_c: 50.0,
            vibration_mm_per_s: 1.0,
            partial_discharge_pc: 5.0,
            oil_temperature_c: 45.0,
            dissolved_gas_h2_ppm: 5.0,
            sf6_pressure_bar: 6.0,
        };
        assert_eq!(CbmAssessor::assess_from_telemetry(&t), RiskLevel::Low);
    }
}
