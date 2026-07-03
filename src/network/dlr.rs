//! Dynamic Line Rating (DLR) — IEEE 738-2012 heat balance model.
//!
//! Computes the maximum allowable current (ampacity) of an overhead
//! transmission conductor under real-time weather conditions, replacing the
//! conservative static rating with a weather-based dynamic value.
//!
//! # Heat Balance (IEEE 738-2012)
//!
//! At steady state the conductor temperature `T_c` satisfies:
//!
//! ```text
//! P_joule + P_solar = P_convection + P_radiation
//! ```
//!
//! where:
//!
//! | Term | Formula | Description |
//! |------|---------|-------------|
//! | `P_joule`   | I² · R(T_c)                           | Resistive heating \[W/m\] |
//! | `P_solar`   | α · D · Q_se                          | Solar heat gain \[W/m\] |
//! | `P_conv`    | (forced + natural) convection          | Convective cooling \[W/m\] |
//! | `P_rad`     | ε · π · D · σ · (T_c⁴ − T_a⁴)        | Radiative cooling \[W/m\] |
//!
//! The ampacity `I_max` is found by solving for `I` that produces
//! `T_c = T_max` (binary search on the heat balance equation).
//!
//! # References
//! - IEEE Std 738-2012, "Standard for Calculating the Current-Temperature of
//!   Bare Overhead Conductors"
//! - CIGRE TB 601, "Guide for Thermal Rating Calculations of Overhead Lines", 2014
//! - Morgan, V.T., "The Thermal Rating of Overhead-Line Conductors", 1982

use serde::{Deserialize, Serialize};

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors produced by the DLR module.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DlrError {
    /// A configuration parameter is outside its valid range.
    #[error("invalid DLR parameter: {0}")]
    InvalidParameter(String),

    /// Binary search failed to converge (heat balance unsolvable).
    #[error("heat balance did not converge: {0}")]
    ConvergenceFailure(String),

    /// No segments provided for a spatial computation.
    #[error("no DLR segments provided")]
    NoSegments,
}

// ── Physical constants ────────────────────────────────────────────────────────

/// Stefan–Boltzmann constant \[W/(m²·K⁴)\].
const SIGMA_SB: f64 = 5.670_374_419e-8;

/// Standard atmosphere pressure at sea level \[Pa\].
const P_ATM_SEA: f64 = 101_325.0;

// ── Conductor specification ───────────────────────────────────────────────────

/// Conductor physical and electrical characteristics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorSpec {
    /// Conductor name (e.g., "ACSR Drake").
    pub name: String,

    /// Outer diameter \[m\].
    pub diameter_m: f64,

    /// AC resistance at `reference_temp_c` \[Ω/km\].
    pub resistance_ohm_per_km: f64,

    /// Temperature coefficient of resistance \[1/°C\].
    ///
    /// R(T) = R_ref · (1 + α · (T − T_ref))
    pub resistance_temp_coeff: f64,

    /// Surface emissivity (0.2–0.9 for weathered conductors).
    pub emissivity: f64,

    /// Solar absorptivity (0.2–0.9; new ≈ 0.5, weathered ≈ 0.9).
    pub solar_absorptivity: f64,

    /// Maximum allowable conductor temperature \[°C\] (e.g., 75 for ACSR).
    pub max_operating_temp_c: f64,

    /// Reference temperature for rated resistance \[°C\] (typically 20 or 25).
    pub reference_temp_c: f64,
}

impl ConductorSpec {
    /// Standard ACSR Drake conductor (795 kcmil).
    pub fn acsr_drake() -> Self {
        Self {
            name: "ACSR Drake 795 kcmil".into(),
            diameter_m: 0.02814,
            resistance_ohm_per_km: 0.07283,
            resistance_temp_coeff: 0.00403,
            emissivity: 0.5,
            solar_absorptivity: 0.5,
            max_operating_temp_c: 75.0,
            reference_temp_c: 25.0,
        }
    }

    /// Resistance at temperature `t_c` \[°C\] in \[Ω/m\].
    pub fn resistance_at_temp(&self, t_c: f64) -> f64 {
        let r_ref = self.resistance_ohm_per_km / 1000.0; // Ω/m
        r_ref * (1.0 + self.resistance_temp_coeff * (t_c - self.reference_temp_c))
    }
}

// ── Weather conditions ────────────────────────────────────────────────────────

/// Ambient weather conditions at a line location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditions {
    /// Ambient air temperature \[°C\].
    pub ambient_temp_c: f64,

    /// Wind speed (perpendicular component) \[m/s\].
    pub wind_speed_ms: f64,

    /// Wind direction angle relative to line axis \[°\] (0 = parallel, 90 = perpendicular).
    pub wind_angle_deg: f64,

    /// Global horizontal solar irradiance \[W/m²\].
    pub solar_irradiance_w_m2: f64,

    /// Altitude above sea level \[m\] (affects air density).
    pub elevation_m: f64,

    /// Local time of day \[h\] (0–24, used for solar position estimate).
    pub time_of_day_h: f64,

    /// Site latitude \[°\] (positive = north).
    pub latitude_deg: f64,
}

impl Default for WeatherConditions {
    fn default() -> Self {
        Self {
            ambient_temp_c: 25.0,
            wind_speed_ms: 0.61, // CIGRE reference: 0.61 m/s
            wind_angle_deg: 45.0,
            solar_irradiance_w_m2: 1000.0,
            elevation_m: 0.0,
            time_of_day_h: 12.0,
            latitude_deg: 40.0,
        }
    }
}

// ── DLR result ────────────────────────────────────────────────────────────────

/// Results of a dynamic line rating computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlrResult {
    /// Conventional static rating (based on default weather) \[A\].
    pub static_rating_a: f64,

    /// Weather-based dynamic ampacity \[A\].
    pub dynamic_rating_a: f64,

    /// Dynamic rating in \[MW\] at the specified nominal voltage.
    pub dynamic_rating_mw: f64,

    /// Conductor temperature at the dynamic rating current \[°C\].
    ///
    /// Should equal `max_operating_temp_c` within solver tolerance.
    pub conductor_temp_c: f64,

    /// Rating increase vs static rating \[%\]; negative means derating.
    pub rating_increase_pct: f64,

    /// Convective heat loss per unit length at rated current \[W/m\].
    pub convective_heat_loss_w_m: f64,

    /// Radiative heat loss per unit length at rated current \[W/m\].
    pub radiative_heat_loss_w_m: f64,

    /// Solar heat gain per unit length \[W/m\].
    pub solar_heat_gain_w_m: f64,

    /// Resistive (Joule) heat gain at rated current \[W/m\].
    pub resistive_heat_gain_w_m: f64,

    /// Residual heat balance error \[W/m\] (should be ~0).
    pub heat_balance_error: f64,
}

// ── DLR forecaster ────────────────────────────────────────────────────────────

/// DLR forecaster for an overhead line.
///
/// Computes IEEE 738-2012 ampacity ratings and provides spatial/temporal
/// rating profiles for a given line.
pub struct DlrForecaster {
    /// Line length \[km\].
    pub line_length_km: f64,

    /// Nominal operating voltage \[kV\].
    pub nominal_voltage_kv: f64,

    /// Number of spatial segments for distributed weather profiles.
    pub n_segments: usize,
}

/// Aggregated configuration for one DLR computation point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlrConfig {
    /// Conductor specification.
    pub conductor: ConductorSpec,

    /// Ambient weather conditions.
    pub weather: WeatherConditions,

    /// Safety derating factor (1.0 = no derating; <1 = conservative).
    pub safety_margin: f64,
}

impl Default for DlrConfig {
    fn default() -> Self {
        Self {
            conductor: ConductorSpec::acsr_drake(),
            weather: WeatherConditions::default(),
            safety_margin: 1.0,
        }
    }
}

impl DlrForecaster {
    /// Create a new DLR forecaster.
    ///
    /// # Arguments
    ///
    /// - `line_length_km`    — total line length \[km\].
    /// - `nominal_voltage_kv` — nominal line-to-line voltage \[kV\].
    /// - `n_segments`         — number of spatial rating segments.
    pub fn new(line_length_km: f64, nominal_voltage_kv: f64, n_segments: usize) -> Self {
        Self {
            line_length_km,
            nominal_voltage_kv,
            n_segments: n_segments.max(1),
        }
    }

    /// Compute dynamic rating from IEEE 738-2012 heat balance equation.
    ///
    /// Returns the maximum allowable current \[A\] for the specified conductor
    /// and weather, corresponding to `T_c = T_max`.
    pub fn compute_rating(&self, config: &DlrConfig) -> Result<DlrResult, DlrError> {
        validate_config(config)?;

        let cond = &config.conductor;
        let weather = &config.weather;
        let t_max = cond.max_operating_temp_c;
        let t_a = weather.ambient_temp_c;

        if t_max <= t_a {
            return Err(DlrError::InvalidParameter(format!(
                "max_operating_temp_c ({t_max}°C) must exceed ambient_temp_c ({t_a}°C)"
            )));
        }

        // ── Solar heat gain ───────────────────────────────────────────────────
        let p_solar = solar_heat_gain(cond, weather);

        // ── Static rating: compute under CIGRE/IEEE reference conditions ──────
        let ref_weather = WeatherConditions {
            ambient_temp_c: 40.0,
            wind_speed_ms: 0.61,
            wind_angle_deg: 90.0,
            solar_irradiance_w_m2: 1000.0,
            ..weather.clone()
        };
        let static_current = solve_ampacity(cond, &ref_weather, config.safety_margin)?;

        // ── Dynamic rating: solve heat balance at actual weather ──────────────
        let dynamic_current = solve_ampacity(cond, weather, config.safety_margin)?;

        // ── Heat balance terms at dynamic rating ──────────────────────────────
        let r_tc = cond.resistance_at_temp(t_max);
        let p_joule = dynamic_current * dynamic_current * r_tc; // W/m
        let p_conv = convective_heat_loss(cond, weather, t_max);
        let p_rad = radiative_heat_loss(cond, weather, t_max);
        let heat_balance_error = p_joule + p_solar - p_conv - p_rad;

        let rating_increase_pct = if static_current > 1.0 {
            (dynamic_current - static_current) / static_current * 100.0
        } else {
            0.0
        };

        // Power rating [MW] = sqrt(3) * V * I (3-phase), kV * A = kVA * 1e-3
        let power_factor = 0.95_f64; // typical
        let dynamic_mw =
            1.732_050_808_f64 * self.nominal_voltage_kv * dynamic_current * power_factor / 1000.0;

        Ok(DlrResult {
            static_rating_a: static_current,
            dynamic_rating_a: dynamic_current,
            dynamic_rating_mw: dynamic_mw,
            conductor_temp_c: t_max,
            rating_increase_pct,
            convective_heat_loss_w_m: p_conv,
            radiative_heat_loss_w_m: p_rad,
            solar_heat_gain_w_m: p_solar,
            resistive_heat_gain_w_m: p_joule,
            heat_balance_error,
        })
    }

    /// Bottleneck rating: minimum across all provided segment configurations.
    ///
    /// Returns the DLR result of the limiting (worst-weather) segment.
    pub fn bottleneck_rating(&self, configs: &[DlrConfig]) -> Result<DlrResult, DlrError> {
        if configs.is_empty() {
            return Err(DlrError::NoSegments);
        }
        let mut min_result: Option<DlrResult> = None;
        for config in configs {
            let result = self.compute_rating(config)?;
            let update = min_result
                .as_ref()
                .is_none_or(|best| result.dynamic_rating_a < best.dynamic_rating_a);
            if update {
                min_result = Some(result);
            }
        }
        min_result.ok_or(DlrError::NoSegments)
    }

    /// Compute a time-series of ratings given a weather forecast.
    ///
    /// # Arguments
    ///
    /// - `base_config`       — base conductor and safety margin (weather overridden).
    /// - `weather_forecast`  — weather conditions at each forecast step.
    /// - `dt_hours`          — time step between forecast points \[h\].
    ///
    /// Returns one `DlrResult` per forecast step.
    pub fn forecast_ratings(
        &self,
        base_config: &DlrConfig,
        weather_forecast: &[WeatherConditions],
        _dt_hours: f64,
    ) -> Result<Vec<DlrResult>, DlrError> {
        weather_forecast
            .iter()
            .map(|weather| {
                let config = DlrConfig {
                    conductor: base_config.conductor.clone(),
                    weather: weather.clone(),
                    safety_margin: base_config.safety_margin,
                };
                self.compute_rating(&config)
            })
            .collect()
    }
}

// ── Heat balance sub-functions ────────────────────────────────────────────────

/// Solve for the ampacity \[A\] that produces `T_c = T_max` using binary search.
fn solve_ampacity(
    cond: &ConductorSpec,
    weather: &WeatherConditions,
    safety_margin: f64,
) -> Result<f64, DlrError> {
    let t_max = cond.max_operating_temp_c;
    let p_solar = solar_heat_gain(cond, weather);
    let p_conv = convective_heat_loss(cond, weather, t_max);
    let p_rad = radiative_heat_loss(cond, weather, t_max);

    // Net cooling available for Joule heating
    let p_net_cooling = p_conv + p_rad - p_solar;
    let r_tc = cond.resistance_at_temp(t_max); // Ω/m

    if r_tc <= 0.0 {
        return Err(DlrError::InvalidParameter(
            "conductor resistance is non-positive".into(),
        ));
    }

    // P_joule = I² · R  →  I = sqrt(P_net / R)
    // If net cooling < 0, ambient + solar already exceed T_max even at I=0
    if p_net_cooling < 0.0 {
        // Rating is zero — conductor already at/above T_max at zero current
        return Ok(0.0_f64);
    }

    let i_max = (p_net_cooling / r_tc).sqrt() * safety_margin;

    // Verify with full non-linear check (binary search for accuracy)
    // The formula above is exact for constant R, so refine via bisection
    // to handle non-linearity in convection model.
    let i_refined = bisect_ampacity(cond, weather, t_max, 0.0, i_max * 2.0, 1e-4)?;
    Ok(i_refined * safety_margin)
}

/// Binary search to find `I` that makes `heat_balance(I, T_c) = 0`.
///
/// Residual: `f(I) = P_joule(I) + P_solar - P_conv - P_rad`
/// At `I=0`: `f < 0` (cooling dominates); at `I=I_max`: `f > 0`.
fn bisect_ampacity(
    cond: &ConductorSpec,
    weather: &WeatherConditions,
    t_max: f64,
    mut lo: f64,
    mut hi: f64,
    tol: f64,
) -> Result<f64, DlrError> {
    let p_solar = solar_heat_gain(cond, weather);
    let p_conv = convective_heat_loss(cond, weather, t_max);
    let p_rad = radiative_heat_loss(cond, weather, t_max);
    let r_tc = cond.resistance_at_temp(t_max);

    let residual = |i: f64| -> f64 { i * i * r_tc + p_solar - p_conv - p_rad };

    // Ensure bracket is valid
    let f_lo = residual(lo);
    let f_hi = residual(hi);

    if f_lo > 0.0 {
        // Even at zero current, T_c > T_max — rating is zero
        return Ok(0.0);
    }
    if f_hi < 0.0 {
        // Extend upper bound
        hi *= 10.0;
    }

    for _ in 0..100 {
        let mid = 0.5 * (lo + hi);
        let f_mid = residual(mid);
        if f_mid.abs() < tol || (hi - lo) < tol {
            return Ok(mid);
        }
        if f_mid < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Err(DlrError::ConvergenceFailure(
        "bisection did not converge within 100 iterations".into(),
    ))
}

/// Solar heat gain per unit conductor length \[W/m\].
///
/// P_solar = α · D · Q_se
///
/// where `Q_se` \[W/m²\] is the effective solar irradiance absorbed by the
/// conductor cross-section (accounts for solar elevation via angle correction).
fn solar_heat_gain(cond: &ConductorSpec, weather: &WeatherConditions) -> f64 {
    // Simplified solar elevation angle based on latitude and time of day
    let hour_angle_deg = (weather.time_of_day_h - 12.0) * 15.0; // 15°/h
    let lat_rad = weather.latitude_deg.to_radians();
    let hour_rad = hour_angle_deg.to_radians();
    // Day number approximation: mid-year
    let decl_rad = (-23.45_f64).to_radians(); // winter solstice approx for conservatism
    let sin_elev = lat_rad.sin() * decl_rad.sin() + lat_rad.cos() * decl_rad.cos() * hour_rad.cos();
    let solar_elevation = sin_elev.asin().max(0.0); // clamp to daylight

    // Effective irradiance on conductor (accounts for angle to sun)
    let q_s = weather.solar_irradiance_w_m2 * solar_elevation.sin().max(0.1);
    cond.solar_absorptivity * cond.diameter_m * q_s
}

/// Forced + natural convection heat loss per unit conductor length \[W/m\].
///
/// IEEE 738-2012 uses two forced convection formulas (low and high wind speed)
/// and takes the larger of forced vs natural convection.
fn convective_heat_loss(cond: &ConductorSpec, weather: &WeatherConditions, t_c: f64) -> f64 {
    let t_a = weather.ambient_temp_c;
    let delta_t = t_c - t_a;
    let d = cond.diameter_m;

    // Air properties at film temperature T_film = (T_c + T_a) / 2
    let t_film = 0.5 * (t_c + t_a) + 273.15; // K

    // Air density correction for altitude
    let p_atm = P_ATM_SEA * (-weather.elevation_m / 8_434.0_f64).exp();

    // Air properties (polynomial fits from IEEE 738 Annex A)
    // Kinematic viscosity [m²/s] at T_film
    let nu_air = (1.458e-6 * t_film.powf(1.5)) / (t_film + 110.4);
    // Air thermal conductivity [W/(m·°C)]
    let k_air = 2.424e-2 + 7.477e-5 * (t_film - 273.15) - 4.407e-9 * (t_film - 273.15).powi(2);

    // Correct density for altitude via pressure ratio
    let rho_corr = p_atm / P_ATM_SEA;

    // Effective wind speed (perpendicular component)
    let phi_rad = weather.wind_angle_deg.to_radians();
    let v_eff = (weather.wind_speed_ms * phi_rad.sin()).max(0.01); // [m/s]

    // Reynolds number
    let re = (d * v_eff * rho_corr) / nu_air;

    // IEEE 738 forced convection (low Re: Re < 100, high Re: Re ≥ 100)
    let (b1, n1) = if re < 100.0 {
        (0.583, 0.471)
    } else {
        (0.0119, 0.916)
    };
    let p_forced = b1 * re.powf(n1) * k_air * delta_t;

    // Natural convection (IEEE 738 Section 4.4.2)
    let g = 9.81_f64;
    let beta = 1.0 / t_film; // thermal expansion coeff [1/K] for ideal gas
    let gr = g * beta * delta_t.abs() * d.powi(3) / (nu_air * nu_air);
    let p_natural = if gr > 0.0 {
        0.55 * k_air * (gr * 0.71_f64).powf(0.25) * delta_t / d * d
    } else {
        0.0
    };

    p_forced.max(p_natural)
}

/// Radiative heat loss per unit conductor length \[W/m\].
///
/// P_rad = ε · π · D · σ · (T_c⁴ − T_a⁴)
fn radiative_heat_loss(cond: &ConductorSpec, weather: &WeatherConditions, t_c: f64) -> f64 {
    let t_c_k = t_c + 273.15;
    let t_a_k = weather.ambient_temp_c + 273.15;
    cond.emissivity
        * std::f64::consts::PI
        * cond.diameter_m
        * SIGMA_SB
        * (t_c_k.powi(4) - t_a_k.powi(4))
}

/// Validate DLR configuration parameters.
fn validate_config(config: &DlrConfig) -> Result<(), DlrError> {
    let cond = &config.conductor;
    if cond.diameter_m <= 0.0 {
        return Err(DlrError::InvalidParameter(
            "conductor diameter must be positive".into(),
        ));
    }
    if cond.resistance_ohm_per_km <= 0.0 {
        return Err(DlrError::InvalidParameter(
            "conductor resistance must be positive".into(),
        ));
    }
    if cond.emissivity <= 0.0 || cond.emissivity > 1.0 {
        return Err(DlrError::InvalidParameter(
            "emissivity must be in (0, 1]".into(),
        ));
    }
    if cond.solar_absorptivity <= 0.0 || cond.solar_absorptivity > 1.0 {
        return Err(DlrError::InvalidParameter(
            "solar_absorptivity must be in (0, 1]".into(),
        ));
    }
    if config.safety_margin <= 0.0 || config.safety_margin > 1.0 {
        return Err(DlrError::InvalidParameter(
            "safety_margin must be in (0, 1]".into(),
        ));
    }
    if config.weather.wind_speed_ms < 0.0 {
        return Err(DlrError::InvalidParameter(
            "wind speed must be non-negative".into(),
        ));
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn acsr_forecaster() -> DlrForecaster {
        DlrForecaster::new(100.0, 138.0, 5)
    }

    fn cold_windy() -> DlrConfig {
        DlrConfig {
            conductor: ConductorSpec::acsr_drake(),
            weather: WeatherConditions {
                ambient_temp_c: 0.0,
                wind_speed_ms: 5.0,
                wind_angle_deg: 90.0,
                solar_irradiance_w_m2: 200.0,
                elevation_m: 0.0,
                time_of_day_h: 6.0,
                latitude_deg: 50.0,
            },
            safety_margin: 1.0,
        }
    }

    fn hot_calm() -> DlrConfig {
        DlrConfig {
            conductor: ConductorSpec::acsr_drake(),
            weather: WeatherConditions {
                ambient_temp_c: 38.0,
                wind_speed_ms: 0.1,
                wind_angle_deg: 90.0,
                solar_irradiance_w_m2: 1200.0,
                elevation_m: 0.0,
                time_of_day_h: 13.0,
                latitude_deg: 30.0,
            },
            safety_margin: 1.0,
        }
    }

    /// Cold/windy: DLR must exceed static rating.
    #[test]
    fn test_cold_windy_dlr_exceeds_static() {
        let forecaster = acsr_forecaster();
        let result = forecaster.compute_rating(&cold_windy()).unwrap();
        assert!(
            result.dynamic_rating_a > 0.0,
            "Dynamic rating must be positive"
        );
        assert!(
            result.dynamic_rating_a > result.static_rating_a,
            "Cold/windy DLR ({:.1} A) should exceed static ({:.1} A)",
            result.dynamic_rating_a,
            result.static_rating_a
        );
        assert!(
            result.rating_increase_pct > 0.0,
            "Rating increase should be positive: {:.1}%",
            result.rating_increase_pct
        );
    }

    /// Hot/calm: DLR must be less than or equal to static rating.
    #[test]
    fn test_hot_calm_dlr_below_static() {
        let forecaster = acsr_forecaster();
        let result = forecaster.compute_rating(&hot_calm()).unwrap();
        assert!(
            result.dynamic_rating_a <= result.static_rating_a + 1.0, // +1A tolerance
            "Hot/calm DLR ({:.1} A) should not exceed static ({:.1} A)",
            result.dynamic_rating_a,
            result.static_rating_a
        );
    }

    /// Heat balance residual must be near zero.
    #[test]
    fn test_heat_balance_error_small() {
        let forecaster = acsr_forecaster();
        let config = DlrConfig::default();
        let result = forecaster.compute_rating(&config).unwrap();
        assert!(
            result.heat_balance_error.abs() < 1.0, // 1 W/m tolerance
            "Heat balance error {:.6} W/m should be ~0",
            result.heat_balance_error
        );
    }

    /// Forecast: time-varying ratings from weather forecast.
    #[test]
    fn test_forecast_ratings_length() {
        let forecaster = acsr_forecaster();
        let base = DlrConfig::default();
        let forecast: Vec<WeatherConditions> = (0..24)
            .map(|h| WeatherConditions {
                time_of_day_h: h as f64,
                wind_speed_ms: 1.0 + (h as f64 * 0.1),
                ..WeatherConditions::default()
            })
            .collect();
        let results = forecaster.forecast_ratings(&base, &forecast, 1.0).unwrap();
        assert_eq!(results.len(), 24, "Should produce 24 hourly ratings");
        for r in &results {
            assert!(r.dynamic_rating_a > 0.0);
        }
    }

    /// Bottleneck rating: minimum segment rating is returned.
    #[test]
    fn test_bottleneck_minimum_segment() {
        let forecaster = acsr_forecaster();
        let cold = cold_windy();
        let hot = hot_calm();
        let neck = forecaster.bottleneck_rating(&[cold, hot]).unwrap();
        let hot_result = forecaster.compute_rating(&hot_calm()).unwrap();
        assert!(
            (neck.dynamic_rating_a - hot_result.dynamic_rating_a).abs() < 1.0,
            "Bottleneck should match the hot/calm (worse) segment"
        );
    }

    /// Safety margin derating: 0.9 margin gives lower rating than 1.0.
    #[test]
    fn test_safety_margin_derating() {
        let forecaster = acsr_forecaster();
        let config_full = DlrConfig::default();
        let full = forecaster.compute_rating(&config_full).unwrap();
        let config_derated = DlrConfig {
            safety_margin: 0.9,
            ..DlrConfig::default()
        };
        let derated = forecaster.compute_rating(&config_derated).unwrap();
        assert!(
            derated.dynamic_rating_a < full.dynamic_rating_a,
            "Derated ({:.1} A) must be lower than full ({:.1} A)",
            derated.dynamic_rating_a,
            full.dynamic_rating_a
        );
    }

    /// validate_config rejects non-positive conductor diameter.
    #[test]
    fn test_validate_config_bad_diameter() {
        let forecaster = acsr_forecaster();
        let config = DlrConfig {
            conductor: ConductorSpec {
                diameter_m: 0.0,
                ..ConductorSpec::acsr_drake()
            },
            ..DlrConfig::default()
        };
        let err = forecaster
            .compute_rating(&config)
            .expect_err("zero diameter should be rejected");
        assert!(
            matches!(err, DlrError::InvalidParameter(_)),
            "Expected InvalidParameter, got: {err}"
        );
    }

    /// validate_config rejects non-positive conductor resistance.
    #[test]
    fn test_validate_config_bad_resistance() {
        let forecaster = acsr_forecaster();
        let config = DlrConfig {
            conductor: ConductorSpec {
                resistance_ohm_per_km: -1.0,
                ..ConductorSpec::acsr_drake()
            },
            ..DlrConfig::default()
        };
        let err = forecaster
            .compute_rating(&config)
            .expect_err("negative resistance should be rejected");
        assert!(
            matches!(err, DlrError::InvalidParameter(_)),
            "Expected InvalidParameter, got: {err}"
        );
    }

    /// validate_config rejects emissivity outside (0, 1].
    #[test]
    fn test_validate_config_bad_emissivity() {
        let forecaster = acsr_forecaster();
        let config = DlrConfig {
            conductor: ConductorSpec {
                emissivity: 1.5,
                ..ConductorSpec::acsr_drake()
            },
            ..DlrConfig::default()
        };
        let err = forecaster
            .compute_rating(&config)
            .expect_err("emissivity > 1 should be rejected");
        assert!(
            matches!(err, DlrError::InvalidParameter(_)),
            "Expected InvalidParameter, got: {err}"
        );
    }

    /// validate_config rejects invalid safety_margin > 1.0.
    #[test]
    fn test_validate_config_bad_safety_margin() {
        let forecaster = acsr_forecaster();
        let config = DlrConfig {
            safety_margin: 1.5,
            ..DlrConfig::default()
        };
        let err = forecaster
            .compute_rating(&config)
            .expect_err("safety_margin > 1 should be rejected");
        assert!(
            matches!(err, DlrError::InvalidParameter(_)),
            "Expected InvalidParameter, got: {err}"
        );
    }

    /// compute_rating returns error when ambient temp >= max operating temp.
    #[test]
    fn test_ambient_above_max_temp_error() {
        let forecaster = acsr_forecaster();
        // Set ambient to exceed max operating temperature (max_operating_temp_c = 75 by default)
        let config = DlrConfig {
            weather: WeatherConditions {
                ambient_temp_c: 80.0,
                ..WeatherConditions::default()
            },
            ..DlrConfig::default()
        };
        let err = forecaster
            .compute_rating(&config)
            .expect_err("ambient >= max_operating_temp should fail");
        assert!(
            matches!(err, DlrError::InvalidParameter(_)),
            "Expected InvalidParameter, got: {err}"
        );
    }

    /// bottleneck_rating returns NoSegments error for empty slice.
    #[test]
    fn test_bottleneck_empty_segments() {
        let forecaster = acsr_forecaster();
        let err = forecaster
            .bottleneck_rating(&[])
            .expect_err("empty segment list should return NoSegments");
        assert!(
            matches!(err, DlrError::NoSegments),
            "Expected NoSegments, got: {err}"
        );
    }

    /// resistance_at_temp increases with temperature for positive temp coefficient.
    #[test]
    fn test_resistance_at_temp_increases_with_heat() {
        let cond = ConductorSpec::acsr_drake();
        let r_cold = cond.resistance_at_temp(20.0);
        let r_hot = cond.resistance_at_temp(75.0);
        assert!(
            r_hot > r_cold,
            "Resistance at 75°C ({r_hot:.6} Ω/m) must exceed resistance at 20°C ({r_cold:.6} Ω/m)"
        );
        // At reference temp the formula collapses: R(T_ref) = R_ref
        let r_ref = cond.resistance_at_temp(cond.reference_temp_c);
        let expected_ref = cond.resistance_ohm_per_km / 1000.0;
        assert!(
            (r_ref - expected_ref).abs() < 1e-12,
            "resistance_at_temp(T_ref) should equal R_ref, got {r_ref}"
        );
    }

    /// DlrForecaster::new clamps n_segments=0 to 1.
    #[test]
    fn test_forecaster_new_clamps_segments() {
        let f = DlrForecaster::new(50.0, 110.0, 0);
        assert_eq!(f.n_segments, 1, "n_segments=0 must be clamped to 1");
        assert!((f.line_length_km - 50.0).abs() < 1e-10);
        assert!((f.nominal_voltage_kv - 110.0).abs() < 1e-10);
    }
}
