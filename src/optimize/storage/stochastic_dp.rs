/// Stochastic Dynamic Programming (SDP) for optimal battery storage operation
/// under price and renewable uncertainty.
///
/// Implements backward induction to compute the exact value function over a
/// discretised (SoC × time) state space, with Monte Carlo scenario averaging
/// for the expectation operator.  Also provides an Approximate DP (ADP) solver
/// that fits a polynomial regression to Monte Carlo rollouts for large state
/// spaces.
///
/// # References
/// - Powell, W.B. (2011). *Approximate Dynamic Programming*. Wiley.
/// - Bertsekas, D.P. (2012). *Dynamic Programming and Optimal Control*. Athena.
use crate::error::OxiGridError;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// LCG pseudo-random number generator (no `rand` dependency)
// ---------------------------------------------------------------------------

/// 64-bit linear congruential generator (Knuth multiplicative hash constants).
struct Lcg64 {
    state: u64,
}

impl Lcg64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Uniform sample in [0, 1).
    fn next_f64(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // Use upper 53 bits for full double precision
        (self.state >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Standard normal via Box-Muller transform.
    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-10);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * core::f64::consts::PI * u2).cos()
    }
}

// ---------------------------------------------------------------------------
// Core data structures
// ---------------------------------------------------------------------------

/// Configuration for the stochastic DP solver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdpConfig {
    /// Number of SoC discretisation levels (default 100).
    pub n_soc_levels: usize,
    /// Number of action discretisation points in [-P_max, P_max] (default 50).
    pub n_actions: usize,
    /// Minimum state of charge (default 0.1).
    pub soc_min: f64,
    /// Maximum state of charge (default 0.9).
    pub soc_max: f64,
    /// Maximum charge/discharge power \[MW\] (default 1.0).
    pub p_max_mw: f64,
    /// Total energy capacity \[MWh\] (default 4.0).
    pub e_max_mwh: f64,
    /// One-way charge efficiency (default 0.95).
    pub eta_charge: f64,
    /// One-way discharge efficiency (default 0.95).
    pub eta_discharge: f64,
    /// Time-step size \[hours\] (default 1.0).
    pub dt_hours: f64,
    /// Planning horizon in time steps (default 24).
    pub n_horizons: usize,
    /// Terminal value coefficient \[$/MWh\] – reward for energy left at end.
    /// Default: average market price, set to 50.0.
    pub c_terminal: f64,
    /// Number of Monte Carlo scenarios for expectation (default 200).
    pub n_scenarios: usize,
    /// Discount factor γ (default 1.0 – no discounting).
    pub discount_factor: f64,
}

impl Default for SdpConfig {
    fn default() -> Self {
        Self {
            n_soc_levels: 100,
            n_actions: 50,
            soc_min: 0.1,
            soc_max: 0.9,
            p_max_mw: 1.0,
            e_max_mwh: 4.0,
            eta_charge: 0.95,
            eta_discharge: 0.95,
            dt_hours: 1.0,
            n_horizons: 24,
            c_terminal: 50.0,
            n_scenarios: 200,
            discount_factor: 1.0,
        }
    }
}

/// Price and renewable scenario covering one planning horizon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// Electricity price \[$/MWh\] for each time step.
    pub price: Vec<f64>,
    /// Renewable generation \[MW\] for each time step.
    pub renewable: Vec<f64>,
    /// Probability weight (all weights must sum to 1.0).
    pub probability: f64,
}

/// Result of the stochastic DP backward induction.
#[derive(Debug, Clone)]
pub struct SdpResult {
    /// Value function: `value_function\[t\][soc_idx]` — expected cost-to-go from
    /// time `t` and SoC level `soc_idx`.  *Minimisation* convention: lower is better.
    pub value_function: Vec<Vec<f64>>,
    /// Optimal power decision: `policy\[t\][soc_idx]` in \[MW\].
    /// Positive = charging, negative = discharging.
    pub policy: Vec<Vec<f64>>,
    /// Expected total revenue across all scenarios \[$/horizon\].
    pub expected_revenue: f64,
    /// Expected number of equivalent full cycles over the horizon.
    pub expected_cycles: f64,
    /// Marginal SoC probability distribution (n_soc_levels entries, sum = 1).
    pub soc_distribution: Vec<f64>,
}

/// Result of a forward simulation using a given policy.
#[derive(Debug, Clone)]
pub struct ForwardSimResult {
    /// Time axis \[hours\].
    pub time: Vec<f64>,
    /// State of charge at each time step (0–1).
    pub soc: Vec<f64>,
    /// Battery power at each time step \[MW\] (positive = charge).
    pub power: Vec<f64>,
    /// Revenue at each time step \[$/step\].
    pub revenue: Vec<f64>,
    /// Total revenue over the horizon \[$\].
    pub total_revenue: f64,
    /// Total equivalent full cycles over the horizon.
    pub total_cycles: f64,
    /// Time-average SoC.
    pub average_soc: f64,
}

// ---------------------------------------------------------------------------
// Stochastic DP solver
// ---------------------------------------------------------------------------

/// Stochastic dynamic programming solver for battery storage operation.
///
/// Performs backward induction over a discretised (SoC × time) state space.
/// The expectation in the Bellman equation is approximated by scenario averaging.
pub struct StochasticDpSolver {
    pub config: SdpConfig,
}

impl StochasticDpSolver {
    /// Create a new solver from the given configuration.
    pub fn new(config: SdpConfig) -> Self {
        Self { config }
    }

    /// Return the SoC value corresponding to a discretisation index.
    fn soc_from_index(&self, idx: usize) -> f64 {
        let n = self.config.n_soc_levels;
        let soc_min = self.config.soc_min;
        let soc_max = self.config.soc_max;
        if n <= 1 {
            return (soc_min + soc_max) / 2.0;
        }
        soc_min + (soc_max - soc_min) * idx as f64 / (n - 1) as f64
    }

    /// Return the action (power in MW) corresponding to a discretisation index.
    fn action_from_index(&self, idx: usize) -> f64 {
        let n = self.config.n_actions;
        let p_max = self.config.p_max_mw;
        if n <= 1 {
            return 0.0;
        }
        -p_max + 2.0 * p_max * idx as f64 / (n - 1) as f64
    }

    /// Compute next SoC given current SoC and power decision.
    ///
    /// Returns `None` if the resulting SoC would violate bounds.
    fn transition(&self, soc: f64, p_bat_mw: f64) -> Option<f64> {
        let cfg = &self.config;
        let delta = if p_bat_mw >= 0.0 {
            // Charging: energy stored = η_c * P * dt / E_max
            cfg.eta_charge * p_bat_mw * cfg.dt_hours / cfg.e_max_mwh
        } else {
            // Discharging: energy drawn = P * dt / (η_d * E_max)
            p_bat_mw * cfg.dt_hours / (cfg.eta_discharge * cfg.e_max_mwh)
        };
        let soc_next = soc + delta;
        if soc_next < cfg.soc_min - 1e-9 || soc_next > cfg.soc_max + 1e-9 {
            None
        } else {
            Some(soc_next.clamp(cfg.soc_min, cfg.soc_max))
        }
    }

    /// Linearly interpolate the value function at an arbitrary SoC in [soc_min, soc_max].
    fn interpolate_value(&self, vf_t: &[f64], soc: f64) -> f64 {
        let n = self.config.n_soc_levels;
        let soc_min = self.config.soc_min;
        let soc_max = self.config.soc_max;

        // Map soc to continuous index
        let t = if (soc_max - soc_min).abs() < 1e-12 {
            0.0
        } else {
            ((soc - soc_min) / (soc_max - soc_min)) * (n - 1) as f64
        };
        let lo = (t.floor() as usize).min(n.saturating_sub(2));
        let hi = lo + 1;
        let frac = t - lo as f64;
        // Defensive: both indices must be valid
        let v_lo = vf_t.get(lo).copied().unwrap_or(f64::INFINITY);
        let v_hi = vf_t.get(hi).copied().unwrap_or(f64::INFINITY);
        v_lo + frac * (v_hi - v_lo)
    }

    /// Find the index of the nearest SoC level for a given SoC.
    fn soc_to_index(&self, soc: f64) -> usize {
        let n = self.config.n_soc_levels;
        let soc_min = self.config.soc_min;
        let soc_max = self.config.soc_max;
        if n <= 1 {
            return 0;
        }
        let t = ((soc - soc_min) / (soc_max - soc_min)) * (n - 1) as f64;
        (t.round() as usize).min(n - 1)
    }

    /// Validate that all scenarios have consistent length.
    fn validate_scenarios(&self, scenarios: &[Scenario]) -> Result<(), OxiGridError> {
        if scenarios.is_empty() {
            return Err(OxiGridError::InvalidParameter(
                "scenario set must be non-empty".to_owned(),
            ));
        }
        let t = self.config.n_horizons;
        for (i, sc) in scenarios.iter().enumerate() {
            if sc.price.len() != t {
                return Err(OxiGridError::InvalidParameter(format!(
                    "scenario {i}: price length {} != n_horizons {t}",
                    sc.price.len()
                )));
            }
            if sc.renewable.len() != t {
                return Err(OxiGridError::InvalidParameter(format!(
                    "scenario {i}: renewable length {} != n_horizons {t}",
                    sc.renewable.len()
                )));
            }
        }
        Ok(())
    }

    /// Backward induction: compute the value function and optimal policy.
    ///
    /// # Arguments
    /// - `scenarios` — weighted scenario set (probabilities must sum ≈ 1)
    ///
    /// # Returns
    /// [`SdpResult`] containing the value function, policy, and summary statistics.
    pub fn solve_backward(&self, scenarios: &[Scenario]) -> Result<SdpResult, OxiGridError> {
        self.validate_scenarios(scenarios)?;

        let cfg = &self.config;
        let n_soc = cfg.n_soc_levels;
        let n_act = cfg.n_actions;
        let t_max = cfg.n_horizons;
        let gamma = cfg.discount_factor;

        // Normalise scenario probabilities defensively
        let prob_sum: f64 = scenarios.iter().map(|s| s.probability).sum();
        let prob_sum = if prob_sum < 1e-12 { 1.0 } else { prob_sum };

        // Allocate value function and policy tables: index [t][soc_idx]
        // We need t_max + 1 time steps (0..=t_max).
        let mut value_function: Vec<Vec<f64>> = vec![vec![0.0; n_soc]; t_max + 1];
        let mut policy: Vec<Vec<f64>> = vec![vec![0.0; n_soc]; t_max];

        // --- Terminal condition ---
        // V_T(SoC) = -c_terminal * SoC * E_max   (reward for leftover energy)
        #[allow(clippy::needless_range_loop)]
        for s in 0..n_soc {
            let soc = self.soc_from_index(s);
            value_function[t_max][s] = -cfg.c_terminal * soc * cfg.e_max_mwh;
        }

        // --- Backward induction ---
        for t in (0..t_max).rev() {
            let vf_next = &value_function[t + 1].clone();

            for s in 0..n_soc {
                let soc = self.soc_from_index(s);
                let mut best_q = f64::INFINITY;
                let mut best_action = 0.0_f64;

                for a in 0..n_act {
                    let p_bat = self.action_from_index(a);

                    // Compute state transition
                    let soc_next = match self.transition(soc, p_bat) {
                        Some(s_next) => s_next,
                        None => continue, // infeasible action
                    };

                    // Expected immediate cost + discounted continuation
                    // Q(s,a) = E_scenarios[ -π_t * p_bat * dt + γ * V_{t+1}(SoC') ]
                    // Note: cost is negative revenue (minimising cost = maximising revenue).
                    let v_next = self.interpolate_value(vf_next, soc_next);

                    let mut q_expected = 0.0_f64;
                    for sc in scenarios {
                        let price_t = sc.price[t];
                        // Revenue = π_t * (-p_bat) * dt  (negative p_bat means discharging)
                        // Cost (to minimise) = -revenue = π_t * p_bat * dt
                        let immediate_cost = price_t * p_bat * cfg.dt_hours;
                        let q = immediate_cost + gamma * v_next;
                        q_expected += sc.probability / prob_sum * q;
                    }

                    if q_expected < best_q {
                        best_q = q_expected;
                        best_action = p_bat;
                    }
                }

                value_function[t][s] = if best_q.is_finite() {
                    best_q
                } else {
                    // All actions infeasible: hold state (no power)
                    self.interpolate_value(vf_next, soc)
                };
                policy[t][s] = best_action;
            }
        }

        // --- Summary statistics via forward simulation ---
        let mut total_revenue_acc = 0.0_f64;
        let mut total_cycles_acc = 0.0_f64;
        let mut soc_dist = vec![0.0_f64; n_soc];
        let n_sc = scenarios.len() as f64;

        for sc in scenarios {
            let weight = sc.probability / prob_sum;
            let initial_soc = (cfg.soc_min + cfg.soc_max) / 2.0;
            match self.simulate_forward(&policy, sc, initial_soc) {
                Ok(fwd) => {
                    total_revenue_acc += weight * fwd.total_revenue;
                    total_cycles_acc += weight * fwd.total_cycles;
                    // Accumulate SoC distribution
                    for &s in &fwd.soc {
                        let idx = self.soc_to_index(s);
                        soc_dist[idx] += weight / (t_max as f64 * n_sc);
                    }
                }
                Err(_) => { /* skip failed simulations */ }
            }
        }

        // Normalise soc_dist
        let dist_sum: f64 = soc_dist.iter().sum();
        if dist_sum > 1e-12 {
            for v in &mut soc_dist {
                *v /= dist_sum;
            }
        }

        Ok(SdpResult {
            value_function,
            policy,
            expected_revenue: total_revenue_acc,
            expected_cycles: total_cycles_acc,
            soc_distribution: soc_dist,
        })
    }

    /// Forward simulation using the computed policy.
    ///
    /// # Arguments
    /// - `policy`      — `policy[t][soc_idx]` power decisions from `solve_backward`
    /// - `scenario`    — single scenario (price + renewable) to simulate
    /// - `initial_soc` — starting state of charge (0–1)
    pub fn simulate_forward(
        &self,
        policy: &[Vec<f64>],
        scenario: &Scenario,
        initial_soc: f64,
    ) -> Result<ForwardSimResult, OxiGridError> {
        let cfg = &self.config;
        let t_max = cfg.n_horizons;

        if policy.len() < t_max {
            return Err(OxiGridError::InvalidParameter(format!(
                "policy length {} < n_horizons {}",
                policy.len(),
                t_max
            )));
        }
        if scenario.price.len() < t_max {
            return Err(OxiGridError::InvalidParameter(format!(
                "scenario price length {} < n_horizons {}",
                scenario.price.len(),
                t_max
            )));
        }

        let mut time = Vec::with_capacity(t_max + 1);
        let mut soc_trace = Vec::with_capacity(t_max + 1);
        let mut power_trace = Vec::with_capacity(t_max);
        let mut revenue_trace = Vec::with_capacity(t_max);

        let mut soc = initial_soc.clamp(cfg.soc_min, cfg.soc_max);
        time.push(0.0);
        soc_trace.push(soc);

        let mut total_revenue = 0.0_f64;
        let mut energy_discharged = 0.0_f64;

        #[allow(clippy::needless_range_loop)]
        for t in 0..t_max {
            // Look up policy
            let soc_idx = self.soc_to_index(soc);
            let p_bat = policy[t].get(soc_idx).copied().unwrap_or(0.0);

            // Clamp to feasible range
            let p_bat = p_bat.clamp(-cfg.p_max_mw, cfg.p_max_mw);

            // Compute feasible transition (may need to reduce power near SoC bounds)
            let soc_next = match self.transition(soc, p_bat) {
                Some(s_next) => s_next,
                None => {
                    // Power is infeasible: try zero action
                    soc
                }
            };

            // Revenue: positive when discharging (selling at price)
            // revenue = -π_t * p_bat * dt  (positive when p_bat < 0)
            let price_t = scenario.price[t];
            let rev = -price_t * p_bat * cfg.dt_hours;
            total_revenue += rev;
            revenue_trace.push(rev);
            power_trace.push(p_bat);

            if p_bat < 0.0 {
                energy_discharged += p_bat.abs() * cfg.dt_hours;
            }

            soc = soc_next;
            time.push((t + 1) as f64 * cfg.dt_hours);
            soc_trace.push(soc);
        }

        let avg_soc = if soc_trace.is_empty() {
            0.0
        } else {
            soc_trace.iter().sum::<f64>() / soc_trace.len() as f64
        };
        let total_cycles = energy_discharged / cfg.e_max_mwh;

        Ok(ForwardSimResult {
            time,
            soc: soc_trace,
            power: power_trace,
            revenue: revenue_trace,
            total_revenue,
            total_cycles,
            average_soc: avg_soc,
        })
    }

    /// One-step greedy policy: choose power to maximise immediate profit,
    /// ignoring future value.
    ///
    /// Useful as a baseline and for comparison with SDP.
    ///
    /// # Arguments
    /// - `soc`       — current state of charge (0–1)
    /// - `price`     — current electricity price \[$/MWh\]
    /// - `renewable` — current renewable output \[MW\] (informational; not used for SoC update)
    /// - `load`      — current load \[MW\] (informational)
    ///
    /// # Returns
    /// Optimal `P_bat` \[MW\] for this time step.
    pub fn greedy_policy(&self, soc: f64, price: f64, _renewable: f64, _load: f64) -> f64 {
        let cfg = &self.config;
        let n_act = cfg.n_actions;
        let mut best_rev = f64::NEG_INFINITY;
        let mut best_p = 0.0_f64;

        for a in 0..n_act {
            let p_bat = self.action_from_index(a);
            // Check feasibility
            if self.transition(soc, p_bat).is_none() {
                continue;
            }
            // Immediate revenue: -price * p_bat * dt (positive when discharging)
            let rev = -price * p_bat * cfg.dt_hours;
            if rev > best_rev {
                best_rev = rev;
                best_p = p_bat;
            }
        }
        best_p
    }
}

// ---------------------------------------------------------------------------
// Scenario generation
// ---------------------------------------------------------------------------

/// Generate price scenarios using geometric Brownian motion (GBM).
///
/// Each scenario is an independent path of the stochastic process:
/// `π_{t+1} = π_t * exp((μ - σ²/2) * dt + σ * √dt * Z)`, Z ~ N(0,1).
///
/// # Arguments
/// - `n_scenarios`    — number of independent paths
/// - `n_steps`        — time steps per scenario
/// - `initial_price`  — starting price \[$/MWh\]
/// - `drift`          — annualised drift μ (set 0 for martingale)
/// - `volatility`     — annualised volatility σ
/// - `dt`             — time-step \[hours\]
/// - `seed`           — LCG seed for reproducibility
pub fn generate_price_scenarios(
    n_scenarios: usize,
    n_steps: usize,
    initial_price: f64,
    drift: f64,
    volatility: f64,
    dt: f64,
    seed: u64,
) -> Vec<Scenario> {
    let mut rng = Lcg64::new(seed);
    let dt_year = dt / 8760.0; // convert hours to years for annualised params
    let mu_dt = (drift - 0.5 * volatility * volatility) * dt_year;
    let sigma_sqrt_dt = volatility * dt_year.sqrt();

    let mut scenarios = Vec::with_capacity(n_scenarios);
    let prob = 1.0 / n_scenarios as f64;

    for _ in 0..n_scenarios {
        let mut prices = Vec::with_capacity(n_steps);
        let mut price = initial_price;
        for _ in 0..n_steps {
            let z = rng.next_normal();
            price *= (mu_dt + sigma_sqrt_dt * z).exp();
            // Clamp to sensible bounds (avoid numerical blow-up)
            price = price.clamp(0.01, initial_price * 20.0);
            prices.push(price);
        }
        scenarios.push(Scenario {
            price: prices,
            renewable: vec![0.0; n_steps],
            probability: prob,
        });
    }
    scenarios
}

/// Generate correlated price + renewable scenarios.
///
/// Uses Cholesky decomposition of a 2×2 correlation matrix to produce
/// correlated normal increments for the price GBM and the renewable Beta variate.
///
/// # Arguments
/// - `n_scenarios`       — number of scenarios
/// - `n_steps`           — time steps per scenario
/// - `price_params`      — `(initial_price, drift, volatility)` for GBM
/// - `renewable_params`  — `(capacity_mw, shape)` for Beta-scaled renewable
/// - `correlation`       — price–renewable correlation ρ ∈ (-1, 1); typically negative
/// - `dt`                — time-step \[hours\]
/// - `seed`              — LCG seed
pub fn generate_joint_scenarios(
    n_scenarios: usize,
    n_steps: usize,
    price_params: (f64, f64, f64),
    renewable_params: (f64, f64),
    correlation: f64,
    dt: f64,
    seed: u64,
) -> Vec<Scenario> {
    let (initial_price, drift, volatility) = price_params;
    let (capacity_mw, shape) = renewable_params;
    let rho = correlation.clamp(-0.9999, 0.9999);

    // Cholesky factor for 2×2 correlation matrix [[1, ρ],[ρ, 1]]:
    // L = [[1, 0], [ρ, sqrt(1-ρ²)]]
    let l21 = rho;
    let l22 = (1.0 - rho * rho).max(0.0).sqrt();

    let dt_year = dt / 8760.0;
    let mu_dt = (drift - 0.5 * volatility * volatility) * dt_year;
    let sigma_sqrt_dt = volatility * dt_year.sqrt();

    let mut rng = Lcg64::new(seed);
    let prob = 1.0 / n_scenarios as f64;
    let mut scenarios = Vec::with_capacity(n_scenarios);

    for _ in 0..n_scenarios {
        let mut prices = Vec::with_capacity(n_steps);
        let mut renewables = Vec::with_capacity(n_steps);
        let mut price = initial_price;

        for _ in 0..n_steps {
            // Draw two independent standard normals
            let z1 = rng.next_normal();
            let z2 = rng.next_normal();

            // Correlated normals via Cholesky
            let w1 = z1; // price innovation
            let w2 = l21 * z1 + l22 * z2; // renewable-correlated innovation

            // Price GBM step
            price *= (mu_dt + sigma_sqrt_dt * w1).exp();
            price = price.clamp(0.01, initial_price * 20.0);
            prices.push(price);

            // Renewable: map w2 to Beta-scaled value.
            // Convert standard normal to uniform via Φ (erf approximation), then to Beta.
            let u_ren = normal_cdf(w2);
            let ren = beta_quantile(u_ren, shape, capacity_mw);
            renewables.push(ren.clamp(0.0, capacity_mw));
        }

        scenarios.push(Scenario {
            price: prices,
            renewable: renewables,
            probability: prob,
        });
    }
    scenarios
}

/// Approximate CDF of the standard normal using Abramowitz & Stegun rational approximation.
/// Error < 7.5e-8 everywhere.
fn normal_cdf(x: f64) -> f64 {
    // Rational approximation for erfc
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let poly = t
        * (0.319_381_530
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    let pdf = (-0.5 * x * x).exp() / (2.0 * core::f64::consts::PI).sqrt();
    let cdf_pos = 1.0 - pdf * poly;
    if x >= 0.0 {
        cdf_pos
    } else {
        1.0 - cdf_pos
    }
}

/// Map a uniform sample u ∈ (0,1) to a Beta(shape, shape)-scaled value in [0, capacity_mw].
///
/// Uses a simple power-law approximation: u^(1/shape) for symmetric Beta.
fn beta_quantile(u: f64, shape: f64, capacity_mw: f64) -> f64 {
    let u = u.clamp(1e-9, 1.0 - 1e-9);
    // For symmetric Beta(α, α), quantile ≈ 0.5 + (u - 0.5) * correction
    // Simple numerical approximation: use Beta CDF inversion via Newton's method
    // with 8 iterations starting from u.
    // For shape >= 1: the mode is at 0.5; use logit-transform approximation.
    let x_init = if shape >= 1.0 {
        // Start at logit-scaled value
        let logit = (u / (1.0 - u)).ln();
        let std_guess = logit / (2.0 * (2.0 * shape - 1.0)).sqrt();
        0.5 + std_guess * 0.5
    } else {
        u.powf(1.0 / shape)
    };
    let x = x_init.clamp(1e-6, 1.0 - 1e-6);
    // Newton refinement on the Beta CDF (using incomplete beta approximation)
    // We do a few steps of bisection instead (more stable).
    let x_refined = bisect_beta_quantile(u, shape, x, 12);
    (x_refined * capacity_mw).clamp(0.0, capacity_mw)
}

/// Bisection search for Beta(α, α) quantile.
fn bisect_beta_quantile(target_u: f64, shape: f64, _x_init: f64, iters: usize) -> f64 {
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    for _ in 0..iters {
        let mid = (lo + hi) / 2.0;
        if regularised_incomplete_beta(mid, shape, shape) < target_u {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// Regularised incomplete beta function I_x(a, a) for symmetric case, via continued fraction.
/// This is an approximation accurate enough for scenario generation purposes.
fn regularised_incomplete_beta(x: f64, a: f64, _b: f64) -> f64 {
    // For symmetric Beta(a, a), I_x(a,a) = 0.5 * I_{2x(1-x)}(a, 0.5)
    // We use a simple series expansion for small a:
    // I_x(a, a) ≈ x^a (1-x)^a / B(a,a) integrated numerically.
    // Simpler: use the normal approximation when shape is large,
    // and direct series for small shape.
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    // Use 16-point Gauss-Legendre quadrature to compute ∫_0^x t^(a-1)(1-t)^(a-1) dt
    // divided by B(a,a).
    // GL abscissae and weights on [-1,1] (8-point rule, transformed to [0, x]):
    // We use a simple 32-step Simpson's rule instead for portability.
    let n_steps = 64_usize;
    let step = x / n_steps as f64;
    let mut integral = 0.0_f64;
    let a_m1 = a - 1.0;

    // Simpson's rule: ∫_0^x f(t) dt
    let f = |t: f64| {
        if t <= 0.0 || t >= 1.0 {
            0.0
        } else {
            (t.ln() * a_m1 + ((1.0 - t).ln() * a_m1)).exp()
        }
    };

    integral += f(0.0);
    let mut i = 1_usize;
    while i < n_steps {
        let t = i as f64 * step;
        let coeff = if i.is_multiple_of(2) { 2.0 } else { 4.0 };
        integral += coeff * f(t);
        i += 1;
    }
    integral += f(x);
    integral *= step / 3.0;

    // Full integral B(a, a) = ∫_0^1 t^(a-1)(1-t)^(a-1) dt
    let full_step = 1.0 / n_steps as f64;
    let mut full_integral = 0.0_f64;
    full_integral += f(0.0);
    let mut j = 1_usize;
    while j < n_steps {
        let t = j as f64 * full_step;
        let coeff = if j.is_multiple_of(2) { 2.0 } else { 4.0 };
        full_integral += coeff * f(t);
        j += 1;
    }
    full_integral += f(1.0);
    full_integral *= full_step / 3.0;

    if full_integral < 1e-300 {
        return 0.5; // degenerate
    }
    (integral / full_integral).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Approximate DP (ADP) with polynomial value function approximation
// ---------------------------------------------------------------------------

/// Approximate Dynamic Programming solver with polynomial basis regression.
///
/// Fits a parametric value function `V_t(SoC) ≈ θ_t · φ(SoC)` where `φ` is a
/// polynomial basis, using Monte Carlo rollouts (LSPI / fitted value iteration).
pub struct AdpSolver {
    pub config: SdpConfig,
    /// Degree of the polynomial basis for SoC features (default 3).
    pub basis_degree: usize,
    /// Number of training scenarios for regression (default 1000).
    pub n_training_scenarios: usize,
    // Fitted weights: outer index = time step, inner = basis coefficients.
    weights: Vec<Vec<f64>>,
}

impl AdpSolver {
    /// Create a new ADP solver.
    pub fn new(config: SdpConfig) -> Self {
        let basis_degree = 3;
        let n_training_scenarios = 1000;
        let n_t = config.n_horizons + 1;
        let n_basis = basis_degree + 1; // polynomial degree d → d+1 coefficients
        let weights = vec![vec![0.0; n_basis]; n_t];
        Self {
            config,
            basis_degree,
            n_training_scenarios,
            weights,
        }
    }

    /// Number of basis functions (polynomial degree + 1).
    fn n_basis(&self) -> usize {
        self.basis_degree + 1
    }

    /// Evaluate the polynomial basis at a given SoC.
    fn basis(&self, soc: f64) -> Vec<f64> {
        // Normalise SoC to [-1, 1] for numerical stability (Chebyshev-like)
        let soc_min = self.config.soc_min;
        let soc_max = self.config.soc_max;
        let x = if (soc_max - soc_min).abs() < 1e-12 {
            0.0
        } else {
            2.0 * (soc - soc_min) / (soc_max - soc_min) - 1.0
        };
        let mut phi = Vec::with_capacity(self.n_basis());
        let mut xp = 1.0_f64;
        for _ in 0..self.n_basis() {
            phi.push(xp);
            xp *= x;
        }
        phi
    }

    /// Dot product of basis and weights.
    fn dot(phi: &[f64], w: &[f64]) -> f64 {
        phi.iter().zip(w.iter()).map(|(a, b)| a * b).sum()
    }

    /// Ordinary Least Squares: solve (Φᵀ Φ) w = Φᵀ y via Cholesky.
    /// Falls back to ridge regression (λ=1e-6) for numerical stability.
    fn ols(phi_mat: &[Vec<f64>], y: &[f64], n_basis: usize) -> Vec<f64> {
        // Build normal equations: A = ΦᵀΦ, b = Φᵀy
        let n = phi_mat.len();
        let mut a = vec![vec![0.0_f64; n_basis]; n_basis];
        let mut b = vec![0.0_f64; n_basis];
        let ridge = 1e-4; // ridge regularisation

        for (row, y_i) in phi_mat.iter().zip(y.iter()) {
            for i in 0..n_basis {
                b[i] += row[i] * y_i;
                for j in 0..n_basis {
                    a[i][j] += row[i] * row[j];
                }
            }
        }
        // Add ridge
        #[allow(clippy::needless_range_loop)]
        for i in 0..n_basis {
            a[i][i] += ridge * n as f64;
        }

        // Cholesky decomposition of a (symmetric positive definite)
        // L L^T = a
        let mut l = vec![vec![0.0_f64; n_basis]; n_basis];
        #[allow(clippy::needless_range_loop)]
        for i in 0..n_basis {
            for j in 0..=i {
                let mut s = a[i][j];
                #[allow(clippy::needless_range_loop)]
                for k in 0..j {
                    s -= l[i][k] * l[j][k];
                }
                if i == j {
                    l[i][j] = s.max(0.0).sqrt();
                } else if l[j][j].abs() > 1e-15 {
                    l[i][j] = s / l[j][j];
                }
            }
        }

        // Forward substitution: L z = b
        let mut z = vec![0.0_f64; n_basis];
        for i in 0..n_basis {
            let mut s = b[i];
            for k in 0..i {
                s -= l[i][k] * z[k];
            }
            if l[i][i].abs() > 1e-15 {
                z[i] = s / l[i][i];
            }
        }

        // Back substitution: L^T w = z
        let mut w = vec![0.0_f64; n_basis];
        for i in (0..n_basis).rev() {
            let mut s = z[i];
            for k in (i + 1)..n_basis {
                s -= l[k][i] * w[k];
            }
            if l[i][i].abs() > 1e-15 {
                w[i] = s / l[i][i];
            }
        }
        w
    }

    /// Fit the ADP value function using Monte Carlo rollouts and regression.
    ///
    /// Algorithm (fitted value iteration):
    /// 1. Generate `n_training_scenarios` rollouts using a random policy.
    /// 2. For each time step t (backward), regress V_{t+1} targets onto basis.
    /// 3. Update weights for time t.
    ///
    /// # Returns
    /// Flattened weight vector (all time steps concatenated).
    pub fn fit(&mut self, scenarios: &[Scenario]) -> Result<Vec<f64>, OxiGridError> {
        if scenarios.is_empty() {
            return Err(OxiGridError::InvalidParameter(
                "ADP fit requires at least one scenario".to_owned(),
            ));
        }

        let cfg = &self.config;
        let t_max = cfg.n_horizons;
        let n_basis = self.n_basis();
        let gamma = cfg.discount_factor;
        let prob_sum: f64 = scenarios.iter().map(|s| s.probability).sum();
        let prob_sum = if prob_sum < 1e-12 { 1.0 } else { prob_sum };

        // Initialise weights: terminal condition V_T(SoC) = -c_terminal * SoC * E_max
        // Polynomial fit at t_max: w such that w · φ(SoC) ≈ -c * SoC * E_max
        let n_soc_pts = 20_usize;
        {
            let mut phi_mat = Vec::with_capacity(n_soc_pts);
            let mut y_vals = Vec::with_capacity(n_soc_pts);
            for k in 0..n_soc_pts {
                let soc =
                    cfg.soc_min + (cfg.soc_max - cfg.soc_min) * k as f64 / (n_soc_pts - 1) as f64;
                phi_mat.push(self.basis(soc));
                y_vals.push(-cfg.c_terminal * soc * cfg.e_max_mwh);
            }
            self.weights[t_max] = Self::ols(&phi_mat, &y_vals, n_basis);
        }

        // Backward pass: for each t, collect (soc, target) pairs and regress
        for t in (0..t_max).rev() {
            let mut phi_mat: Vec<Vec<f64>> = Vec::new();
            let mut y_vals: Vec<f64> = Vec::new();

            // Sample SoC points
            let n_soc_pts = 50_usize.max(self.n_training_scenarios / t_max.max(1));
            let w_next = self.weights[t + 1].clone();

            for sc in scenarios {
                let sc_weight = sc.probability / prob_sum;
                let price_t = sc.price.get(t).copied().unwrap_or(cfg.c_terminal);

                for k in 0..n_soc_pts {
                    let soc = cfg.soc_min
                        + (cfg.soc_max - cfg.soc_min) * k as f64 / n_soc_pts.max(1) as f64;

                    // Compute best Q-value at this (t, soc) for this scenario
                    let mut best_q = f64::INFINITY;
                    let n_act = cfg.n_actions;
                    let p_max = cfg.p_max_mw;

                    for a in 0..n_act {
                        let p_bat = if n_act <= 1 {
                            0.0
                        } else {
                            -p_max + 2.0 * p_max * a as f64 / (n_act - 1) as f64
                        };

                        let delta = if p_bat >= 0.0 {
                            cfg.eta_charge * p_bat * cfg.dt_hours / cfg.e_max_mwh
                        } else {
                            p_bat * cfg.dt_hours / (cfg.eta_discharge * cfg.e_max_mwh)
                        };
                        let soc_next = soc + delta;
                        if soc_next < cfg.soc_min - 1e-9 || soc_next > cfg.soc_max + 1e-9 {
                            continue;
                        }
                        let soc_next = soc_next.clamp(cfg.soc_min, cfg.soc_max);
                        let phi_next = self.basis(soc_next);
                        let v_next = Self::dot(&phi_next, &w_next);
                        let immediate = price_t * p_bat * cfg.dt_hours;
                        let q = immediate + gamma * v_next;
                        if q < best_q {
                            best_q = q;
                        }
                    }
                    if best_q.is_finite() {
                        phi_mat.push(self.basis(soc));
                        y_vals.push(sc_weight * best_q);
                    }
                }
            }

            if !phi_mat.is_empty() {
                // Aggregate y values for same soc points (average over scenarios)
                self.weights[t] = Self::ols(&phi_mat, &y_vals, n_basis);
            } else {
                // No feasible actions: keep zero weights
                self.weights[t] = vec![0.0; n_basis];
            }
        }

        // Return flattened weights
        let flat: Vec<f64> = self.weights.iter().flatten().copied().collect();
        Ok(flat)
    }

    /// Evaluate the approximate value function at a given SoC and time step.
    pub fn evaluate(&self, weights: &[f64], soc: f64, t: usize) -> f64 {
        let n_basis = self.n_basis();
        let t_clamped = t.min(self.config.n_horizons);
        let offset = t_clamped * n_basis;

        if offset + n_basis > weights.len() {
            // Fall back to linear terminal value estimate
            return -self.config.c_terminal * soc * self.config.e_max_mwh;
        }

        let w_slice = &weights[offset..offset + n_basis];
        let phi = self.basis(soc);
        Self::dot(&phi, w_slice)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config_small() -> SdpConfig {
        SdpConfig {
            n_soc_levels: 20,
            n_actions: 10,
            n_horizons: 24,
            n_scenarios: 5,
            ..Default::default()
        }
    }

    fn flat_price_scenario(price: f64) -> Scenario {
        Scenario {
            price: vec![price; 24],
            renewable: vec![0.0; 24],
            probability: 1.0,
        }
    }

    #[test]
    fn test_sdp_value_function_shape() {
        let config = default_config_small();
        let scenarios = vec![flat_price_scenario(100.0)];
        let solver = StochasticDpSolver::new(config.clone());
        let result = solver.solve_backward(&scenarios).expect("SDP failed");

        // V has correct shape: (n_horizons+1) × n_soc_levels
        assert_eq!(result.value_function.len(), config.n_horizons + 1);
        assert_eq!(result.value_function[0].len(), config.n_soc_levels);

        // At t=0 with uniformly high prices, higher SoC should be more valuable
        // (more energy to discharge = lower cost = lower value in minimisation sense).
        let v0_low = result.value_function[0][0];
        let v0_high = result.value_function[0][config.n_soc_levels - 1];
        // Higher SoC gives lower (more negative) cost-to-go when price is high.
        assert!(
            v0_high <= v0_low,
            "Higher SoC should have lower (or equal) cost-to-go at high price: v_high={v0_high:.2}, v_low={v0_low:.2}"
        );
    }

    #[test]
    fn test_sdp_policy_discharge_at_high_price() {
        // Single scenario: flat high price (100 $/MWh).
        // With a high price and ample SoC, the policy should favour discharging.
        let n_h = 24;
        let config = SdpConfig {
            n_soc_levels: 20,
            n_actions: 11, // includes 0
            n_horizons: n_h,
            n_scenarios: 1,
            ..Default::default()
        };
        let scenarios = vec![flat_price_scenario(200.0)]; // very high price
        let solver = StochasticDpSolver::new(config.clone());
        let result = solver.solve_backward(&scenarios).expect("SDP failed");

        // At t=0 with high SoC, the policy should discharge (negative power).
        let high_soc_idx = config.n_soc_levels - 1;
        let p = result.policy[0][high_soc_idx];
        assert!(
            p <= 0.0,
            "At high SoC and high price, policy should discharge: p={p:.3}"
        );
    }

    #[test]
    fn test_forward_sim_soc_bounds() {
        let config = default_config_small();
        let scenarios = vec![flat_price_scenario(50.0)];
        let solver = StochasticDpSolver::new(config.clone());
        let result = solver.solve_backward(&scenarios).expect("SDP failed");

        let sc = flat_price_scenario(50.0);
        let initial_soc = (config.soc_min + config.soc_max) / 2.0;
        let fwd = solver
            .simulate_forward(&result.policy, &sc, initial_soc)
            .expect("Forward sim failed");

        for &s in &fwd.soc {
            assert!(
                (config.soc_min - 1e-6..=config.soc_max + 1e-6).contains(&s),
                "SoC out of bounds: {s:.4}"
            );
        }
        assert_eq!(fwd.time.len(), config.n_horizons + 1);
        assert_eq!(fwd.power.len(), config.n_horizons);
    }

    #[test]
    fn test_scenario_generation() {
        let n_sc = 10;
        let scenarios = generate_price_scenarios(n_sc, 24, 50.0, 0.0, 0.2, 1.0, 42);

        assert_eq!(
            scenarios.len(),
            n_sc,
            "Should generate exactly n_sc scenarios"
        );

        for sc in &scenarios {
            assert_eq!(sc.price.len(), 24);
            for &p in &sc.price {
                assert!(p > 0.0, "All prices should be positive, got {p}");
            }
        }

        let prob_sum: f64 = scenarios.iter().map(|s| s.probability).sum();
        assert!(
            (prob_sum - 1.0).abs() < 1e-9,
            "Probabilities should sum to 1.0, got {prob_sum}"
        );
    }

    #[test]
    fn test_sdp_vs_greedy_revenue() {
        // On a simple 24h scenario, SDP-optimal policy should achieve
        // at least as good revenue as the greedy one-step policy.
        let config = SdpConfig {
            n_soc_levels: 20,
            n_actions: 11,
            n_horizons: 24,
            n_scenarios: 1,
            c_terminal: 0.0, // no terminal value so comparison is fair
            ..Default::default()
        };

        // Scenario: low price first half, high price second half
        let mut prices = vec![20.0_f64; 12];
        prices.extend(vec![80.0_f64; 12]);
        let sc = Scenario {
            price: prices,
            renewable: vec![0.0; 24],
            probability: 1.0,
        };

        let solver = StochasticDpSolver::new(config.clone());
        let sdp_result = solver
            .solve_backward(std::slice::from_ref(&sc))
            .expect("SDP failed");
        let sdp_fwd = solver
            .simulate_forward(&sdp_result.policy, &sc, 0.5)
            .expect("SDP forward sim failed");

        // Greedy simulation
        let mut soc = 0.5_f64;
        let mut greedy_revenue = 0.0_f64;
        for t in 0..config.n_horizons {
            let price = sc.price[t];
            let p_bat = solver.greedy_policy(soc, price, 0.0, 0.0);
            let delta = if p_bat >= 0.0 {
                config.eta_charge * p_bat * config.dt_hours / config.e_max_mwh
            } else {
                p_bat * config.dt_hours / (config.eta_discharge * config.e_max_mwh)
            };
            let soc_next = (soc + delta).clamp(config.soc_min, config.soc_max);
            greedy_revenue += -price * p_bat * config.dt_hours;
            soc = soc_next;
        }

        // SDP should be at least as good as greedy (within reasonable tolerance)
        assert!(
            sdp_fwd.total_revenue >= greedy_revenue - 1e-3,
            "SDP revenue {:.4} should be >= greedy revenue {:.4}",
            sdp_fwd.total_revenue,
            greedy_revenue
        );
    }

    #[test]
    fn test_adp_fit_predict() {
        let config = SdpConfig {
            n_soc_levels: 10,
            n_actions: 5,
            n_horizons: 6,
            n_scenarios: 3,
            ..Default::default()
        };
        let scenarios: Vec<Scenario> = (0..3)
            .map(|i| Scenario {
                price: vec![30.0 + 10.0 * i as f64; 6],
                renewable: vec![0.0; 6],
                probability: 1.0 / 3.0,
            })
            .collect();

        let mut adp = AdpSolver::new(config.clone());
        let weights = adp.fit(&scenarios).expect("ADP fit failed");

        // Weights should be non-empty
        assert!(!weights.is_empty(), "Weights should be non-empty");

        // Evaluate at various SoC levels
        let soc_mid = (config.soc_min + config.soc_max) / 2.0;
        let v_mid = adp.evaluate(&weights, soc_mid, 0);
        assert!(v_mid.is_finite(), "ADP value should be finite at mid SoC");

        // Check approximate monotonicity: higher SoC → lower cost at t=0
        // (with high enough prices, more SoC = more discharge potential = better)
        let v_lo = adp.evaluate(&weights, config.soc_min + 0.01, 0);
        let v_hi = adp.evaluate(&weights, config.soc_max - 0.01, 0);
        // Allow some tolerance since it is an approximation
        assert!(
            v_hi <= v_lo + (v_lo.abs() * 0.5 + 1.0),
            "ADP: v_hi={v_hi:.3} should not greatly exceed v_lo={v_lo:.3}"
        );
    }

    #[test]
    fn test_lcg_reproducibility() {
        let s1 = generate_price_scenarios(10, 24, 50.0, 0.0, 0.2, 1.0, 42);
        let s2 = generate_price_scenarios(10, 24, 50.0, 0.0, 0.2, 1.0, 42);

        assert_eq!(
            s1.len(),
            s2.len(),
            "Both runs should produce same number of scenarios"
        );
        for (i, (a, b)) in s1.iter().zip(s2.iter()).enumerate() {
            assert_eq!(
                a.price[0], b.price[0],
                "Scenario {i} price[0] should match: {} vs {}",
                a.price[0], b.price[0]
            );
        }
    }

    #[test]
    fn test_joint_scenario_generation() {
        let scenarios =
            generate_joint_scenarios(20, 24, (50.0, 0.0, 0.3), (10.0, 2.0), -0.6, 1.0, 123);
        assert_eq!(scenarios.len(), 20);

        let prob_sum: f64 = scenarios.iter().map(|s| s.probability).sum();
        assert!((prob_sum - 1.0).abs() < 1e-9, "Probs sum to 1: {prob_sum}");

        for sc in &scenarios {
            assert_eq!(sc.price.len(), 24);
            assert_eq!(sc.renewable.len(), 24);
            for &r in &sc.renewable {
                assert!(
                    (0.0..=10.0).contains(&r),
                    "Renewable out of [0, capacity]: {r}"
                );
            }
        }
    }

    #[test]
    fn test_terminal_value_decreasing_cost() {
        // Terminal: V_T(SoC) = -c_terminal * SoC * E_max.
        // Higher SoC → more negative value (better in minimisation sense).
        let config = default_config_small();
        let scenarios = vec![flat_price_scenario(0.0)]; // zero price: only terminal matters
        let solver = StochasticDpSolver::new(config.clone());
        let result = solver.solve_backward(&scenarios).expect("SDP failed");

        let t_end = config.n_horizons;
        let v_low = result.value_function[t_end][0];
        let v_high = result.value_function[t_end][config.n_soc_levels - 1];
        assert!(
            v_high < v_low,
            "Terminal: higher SoC should give lower (more negative) value: v_high={v_high:.3}, v_low={v_low:.3}"
        );
    }

    #[test]
    fn test_greedy_policy_is_valid() {
        let config = default_config_small();
        let solver = StochasticDpSolver::new(config.clone());

        let soc_mid = (config.soc_min + config.soc_max) / 2.0;
        let p = solver.greedy_policy(soc_mid, 100.0, 5.0, 10.0);

        // At high price: greedy should discharge (negative) or hold
        assert!(
            p <= 0.0,
            "At high price and mid-SoC, greedy should discharge or hold: p={p:.3}"
        );
        assert!(
            p.abs() <= config.p_max_mw + 1e-9,
            "Greedy power should not exceed p_max: {p:.3}"
        );
    }
}
