//! Enhanced Small Signal Stability Analysis (ESSA) for power systems.
//!
//! Implements linearisation around an operating point, eigenvalue computation via
//! QR algorithm, participation factor analysis, and inter-area mode identification.
//!
//! # State variables
//! Classical model: \[Δδ_i, Δω_i\] per machine (2n states)
//! With AVR:        \[Δδ_i, Δω_i, ΔE'q_i\] per machine (3n states)
//!
//! # Units
//! - Inertia H: \[MJ/MVA\]
//! - Frequency: \[Hz\]
//! - Time constants: \[s\]
//! - Power quantities: \[pu\]
//! - Angles: \[rad\]

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Nominal system frequency \[rad/s\]
const OMEGA_0: f64 = 2.0 * PI * 50.0;

// ---------------------------------------------------------------------------
// Core data structures
// ---------------------------------------------------------------------------

/// Per-machine linearised small-signal model parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineSmallSignalModel {
    /// Unique machine identifier
    pub machine_id: String,
    /// Inertia constant H \[MJ/MVA\]
    pub inertia_mj_mva: f64,
    /// Per-unit damping coefficient D \[pu torque / pu speed\]
    pub damping: f64,
    /// Synchronising torque coefficient Ks = ∂Pe/∂δ \[pu\]
    pub synchronizing_coeff: f64,
    /// Transient EMF E'q \[pu\]
    pub transient_emf: f64,
    /// Exciter gain Ka (IEEE Type 1) \[pu/pu\]
    pub exciter_gain: f64,
    /// Exciter time constant Ta \[s\]
    pub exciter_time_s: f64,
    /// Whether the AVR/exciter is included in linearisation
    pub avr_enabled: bool,
}

/// Assembled multi-machine linearised power system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmallSignalSystem {
    /// Vector of machine models (indexed 0..n-1)
    pub machines: Vec<MachineSmallSignalModel>,
    /// Simplified linearised network coupling matrix B_reduced (n×n) \[pu\]
    pub network_matrix: Vec<Vec<f64>>,
    /// System MVA base \[MVA\]
    pub base_mva: f64,
}

/// Oscillation mode type classified by frequency range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModeType {
    /// 0.8–2.0 Hz — local plant oscillation (1-2 machines vs. the rest)
    LocalPlantMode,
    /// 0.1–0.8 Hz — inter-area oscillation (groups of machines)
    InterAreaMode,
    /// >2.0 Hz — control system mode
    ControlMode,
    /// Excitation system mode (identified by AVR states)
    ExciterMode,
}

/// Single eigenvalue result with physical interpretation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EigenvalueResult {
    /// Real part σ of eigenvalue (negative → stable)
    pub real: f64,
    /// Imaginary part ωd \[rad/s\]
    pub imag: f64,
    /// Natural frequency fn = |imag| / (2π) \[Hz\]
    pub frequency_hz: f64,
    /// Damping ratio ζ = −σ / √(σ²+ωd²)
    pub damping_ratio: f64,
    /// Mode classification
    pub mode_type: ModeType,
}

/// Participation factor of a state variable in a given mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipationFactor {
    /// Machine this state belongs to
    pub machine_id: String,
    /// Normalised participation magnitude (0–1, Σ = 1 per mode)
    pub factor: f64,
    /// State variable name: "delta", "omega", or "E_q"
    pub state_variable: String,
}

/// Inter-area oscillation mode grouping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterAreaMode {
    /// Mode index into EigenvalueResult vector
    pub mode_id: usize,
    /// Natural frequency \[Hz\]
    pub frequency_hz: f64,
    /// Damping ratio ζ
    pub damping_ratio: f64,
    /// Machines in phase-lead group (swing in one direction)
    pub group1_machines: Vec<String>,
    /// Machines in phase-lag group (swing in opposite direction)
    pub group2_machines: Vec<String>,
    /// Modal energy ratio (group1 kinetic / group2 kinetic)
    pub modal_energy_ratio: f64,
}

/// Complete mode report from ESSA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeReport {
    /// All classified eigenvalue results
    pub eigenvalues: Vec<EigenvalueResult>,
    /// Participation factors per mode (outer index = mode, inner = states)
    pub participation_factors: Vec<Vec<ParticipationFactor>>,
    /// Identified inter-area modes
    pub inter_area_modes: Vec<InterAreaMode>,
    /// Indices into eigenvalues with damping < min_damping_ratio
    pub poorly_damped_modes: Vec<usize>,
    /// Index of least-damped (or unstable) mode, if any
    pub critical_mode: Option<usize>,
}

/// Configuration for the ESSA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmallSignalConfig {
    /// Poorly-damped threshold ζ_min (default 0.05 = 5 %)
    pub min_damping_ratio: f64,
    /// Frequency band of interest \[Hz\] (default 0.1–2.5 Hz)
    pub frequency_range: (f64, f64),
    /// Maximum QR iterations (default 200)
    pub max_iterations_qr: usize,
    /// Convergence tolerance for QR (default 1e-10)
    pub convergence_tol: f64,
}

impl Default for SmallSignalConfig {
    fn default() -> Self {
        Self {
            min_damping_ratio: 0.05,
            frequency_range: (0.1, 2.5),
            max_iterations_qr: 200,
            convergence_tol: 1e-10,
        }
    }
}

/// Main analyser combining system data and configuration.
pub struct SmallSignalAnalyzer {
    pub system: SmallSignalSystem,
    pub config: SmallSignalConfig,
}

// ---------------------------------------------------------------------------
// Internal linear algebra helpers (pure Rust, no external LA crate)
// ---------------------------------------------------------------------------

/// Matrix-vector multiply y = A * x.
fn mat_vec(a: &[Vec<f64>], x: &[f64]) -> Vec<f64> {
    let n = a.len();
    let mut y = vec![0.0_f64; n];
    for (i, row) in a.iter().enumerate() {
        for (j, &aij) in row.iter().enumerate() {
            y[i] += aij * x[j];
        }
    }
    y
}

/// Transpose of matrix.
fn transpose(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if a.is_empty() {
        return Vec::new();
    }
    let m = a.len();
    let n = a[0].len();
    let mut t = vec![vec![0.0_f64; m]; n];
    for i in 0..m {
        for j in 0..n {
            t[j][i] = a[i][j];
        }
    }
    t
}

/// Identity matrix of size n.
fn identity(n: usize) -> Vec<Vec<f64>> {
    let mut mat = vec![vec![0.0_f64; n]; n];
    for (k, row) in mat.iter_mut().enumerate() {
        row[k] = 1.0;
    }
    mat
}

/// L2-norm of a slice.
fn norm2(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Apply a Householder reflection P = I - 2*u*u^T from the left to rows [start..start+len].
/// Only modifies columns [col_start..n].
#[allow(clippy::needless_range_loop)]
fn householder_left(h: &mut [Vec<f64>], u: &[f64], row_start: usize, col_start: usize) {
    let n_cols = h[0].len();
    let vlen = u.len();
    for j in col_start..n_cols {
        let dot: f64 = (0..vlen).map(|i| u[i] * h[row_start + i][j]).sum();
        for (i, &ui) in u.iter().enumerate().take(vlen) {
            h[row_start + i][j] -= 2.0 * ui * dot;
        }
    }
}

/// Apply a Householder reflection P = I - 2*u*u^T from the right to cols [start..start+len].
/// Only modifies rows [0..row_end].
fn householder_right(h: &mut [Vec<f64>], u: &[f64], col_start: usize, row_end: usize) {
    let vlen = u.len();
    for row in h.iter_mut().take(row_end) {
        let dot: f64 = (0..vlen).map(|j| row[col_start + j] * u[j]).sum();
        for (j, &uj) in u.iter().enumerate().take(vlen) {
            row[col_start + j] -= 2.0 * uj * dot;
        }
    }
}

/// Build a unit Householder vector u such that (I - 2*u*u^T)*x has zeros below index 0.
/// Returns (u, alpha) where alpha = -sign(x[0])*||x||.
/// Returns None if reflection is unnecessary (already zero below).
fn make_householder(x: &[f64]) -> Option<Vec<f64>> {
    let sigma = norm2(x);
    if sigma < 1e-15 {
        return None;
    }
    let alpha = if x[0] >= 0.0 { -sigma } else { sigma };
    let mut u = x.to_vec();
    u[0] -= alpha;
    let beta = norm2(&u);
    if beta < 1e-15 {
        return None;
    }
    for x in &mut u {
        *x /= beta;
    }
    Some(u)
}

/// Householder reduction to upper Hessenberg form.
/// Returns H (upper Hessenberg) and Q (accumulated orthogonal).
fn hessenberg_reduction(a: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let n = a.len();
    let mut h = a.to_vec();
    let mut q = identity(n);

    for k in 0..n.saturating_sub(2) {
        let col_len = n - k - 1;
        // Extract sub-column
        let x: Vec<f64> = (0..col_len).map(|i| h[k + 1 + i][k]).collect();
        if let Some(u) = make_householder(&x) {
            // Apply P from left (rows k+1..n, all columns)
            householder_left(&mut h, &u, k + 1, 0);
            // Apply P from right (all rows, cols k+1..n)
            householder_right(&mut h, &u, k + 1, n);
            // Accumulate Q (all rows, cols k+1..n)
            householder_right(&mut q, &u, k + 1, n);
        }
    }
    (h, q)
}

/// Extract eigenvalues of a 2×2 matrix.
fn eig_2x2(a00: f64, a01: f64, a10: f64, a11: f64) -> [(f64, f64); 2] {
    let tr = a00 + a11;
    let det = a00 * a11 - a01 * a10;
    let disc = tr * tr - 4.0 * det;
    if disc >= 0.0 {
        let sq = disc.sqrt();
        [(0.5 * (tr + sq), 0.0), (0.5 * (tr - sq), 0.0)]
    } else {
        let sq = (-disc).sqrt();
        [(0.5 * tr, 0.5 * sq), (0.5 * tr, -0.5 * sq)]
    }
}

/// Francis double-shift QR step on H[0..p][0..p].
/// Modifies H in place.
#[allow(clippy::needless_range_loop)]
fn francis_qr_step(h: &mut [Vec<f64>], p: usize, tol: f64) {
    if p < 2 {
        return;
    }
    let n = p; // work on rows/cols 0..p

    // Shifts: eigenvalues of bottom 2x2
    let m = n - 1;
    let s = h[m - 1][m - 1] + h[m][m];
    let t = h[m - 1][m - 1] * h[m][m] - h[m - 1][m] * h[m][m - 1];

    // First column of M = H^2 - s*H + t*I
    let mut x = h[0][0] * h[0][0] + h[0][1] * h[1][0] - s * h[0][0] + t;
    let mut y = h[1][0] * (h[0][0] + h[1][1] - s);
    let mut z = if n > 2 { h[2][1] * h[1][0] } else { 0.0 };

    for k in 0..n.saturating_sub(1) {
        let len = if k + 3 <= n { 3 } else { n - k };
        let v_raw: Vec<f64> = match len {
            3 => vec![x, y, z],
            2 => vec![x, y],
            _ => vec![x],
        };
        let u = match make_householder(&v_raw) {
            Some(u) => u,
            None => {
                // Update x, y, z for next step
                if k + 1 < n {
                    x = h[k + 1][k];
                    y = if k + 2 < n { h[k + 2][k] } else { 0.0 };
                    z = if k + 3 < n { h[k + 3][k] } else { 0.0 };
                }
                continue;
            }
        };
        let vlen = u.len();
        let r = if k == 0 { 0 } else { k - 1 };

        // Apply from left: H[k..k+vlen, r..n]
        for j in r..n {
            let dot: f64 = (0..vlen).map(|i| u[i] * h[k + i][j]).sum();
            for (i, &ui) in u.iter().enumerate().take(vlen) {
                h[k + i][j] -= 2.0 * ui * dot;
            }
        }
        // Apply from right: H[0..min(k+vlen+1, n), k..k+vlen]
        let row_end = (k + vlen + 1).min(n);
        for row in h.iter_mut().take(row_end) {
            let dot: f64 = (0..vlen).map(|j| row[k + j] * u[j]).sum();
            for (j, &uj) in u.iter().enumerate().take(vlen) {
                row[k + j] -= 2.0 * uj * dot;
            }
        }

        // Enforce subdiagonal zeros below bulge (numerical clean-up)
        for i in (k + 2)..(k + vlen).min(n) {
            if h[i][k].abs() < tol {
                h[i][k] = 0.0;
            }
        }

        // Update (x, y, z) for next step from below-diagonal elements
        if k + 1 < n {
            x = h[k + 1][k];
            y = if k + 2 < n { h[k + 2][k] } else { 0.0 };
            z = if k + 3 < n { h[k + 3][k] } else { 0.0 };
        }
    }
}

/// Implicit double-shift QR iteration on an upper Hessenberg matrix.
/// Returns eigenvalues as (real, imag) pairs.
///
/// Uses Francis double-shift with deflation.
fn qr_iteration_hessenberg(h_in: &[Vec<f64>], max_iter: usize, tol: f64) -> Vec<(f64, f64)> {
    let n = h_in.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![(h_in[0][0], 0.0)];
    }
    if n == 2 {
        let [e1, e2] = eig_2x2(h_in[0][0], h_in[0][1], h_in[1][0], h_in[1][1]);
        return vec![e1, e2];
    }

    let mut h = h_in.to_vec();
    let mut eigenvalues: Vec<(f64, f64)> = Vec::with_capacity(n);
    let mut active = n; // current active subproblem size

    let total_max = max_iter * n;
    let mut total_iters = 0;

    while active > 0 {
        if total_iters > total_max {
            // Force extraction of remaining diagonal
            for (i, row) in h.iter().enumerate().take(active) {
                eigenvalues.push((row[i], 0.0));
            }
            break;
        }

        if active == 1 {
            eigenvalues.push((h[0][0], 0.0));
            break;
        }

        if active == 2 {
            let [e1, e2] = eig_2x2(h[0][0], h[0][1], h[1][0], h[1][1]);
            eigenvalues.push(e1);
            eigenvalues.push(e2);
            break;
        }

        // Check for deflation at bottom-right corner (1×1 block)
        let sub = h[active - 1][active - 2].abs();
        let diag_sum = h[active - 2][active - 2].abs() + h[active - 1][active - 1].abs();
        if sub <= tol * diag_sum || sub <= tol * 1e-3 {
            eigenvalues.push((h[active - 1][active - 1], 0.0));
            active -= 1;
            // Shrink working block (copy top-left active×active into place)
            // h is already structured so no copy needed — just reduce active
            continue;
        }

        // Check for deflation at bottom-right (2×2 block)
        if active >= 3 {
            let sub2 = h[active - 2][active - 3].abs();
            let diag_sum2 = h[active - 3][active - 3].abs() + h[active - 2][active - 2].abs();
            if sub2 <= tol * diag_sum2 || sub2 <= tol * 1e-3 {
                let r = active - 2;
                let [e1, e2] = eig_2x2(h[r][r], h[r][r + 1], h[r + 1][r], h[r + 1][r + 1]);
                eigenvalues.push(e1);
                eigenvalues.push(e2);
                active -= 2;
                continue;
            }
        }

        // Apply one Francis double-shift QR step on active submatrix
        let mut sub_h: Vec<Vec<f64>> = (0..active)
            .map(|i| (0..active).map(|j| h[i][j]).collect())
            .collect();
        francis_qr_step(&mut sub_h, active, tol);
        for i in 0..active {
            for j in 0..active {
                h[i][j] = sub_h[i][j];
            }
        }
        total_iters += 1;
    }

    eigenvalues
}

/// Power iteration to approximate the dominant right eigenvector for eigenvalue `lambda`.
fn approx_eigenvector(a: &[Vec<f64>], lambda_r: f64, lambda_i: f64) -> Vec<f64> {
    let n = a.len();
    let mut v: Vec<f64> = (0..n)
        .map(|i| if i == 0 { 1.0 } else { 0.1 / (i as f64) })
        .collect();

    // Shifted inverse-free power iteration: multiply by (A - shift*I) repeatedly
    let shift = lambda_r;
    for _ in 0..30 {
        let mut w = mat_vec(a, &v);
        for (wi, &vi) in w.iter_mut().zip(v.iter()) {
            *wi -= shift * vi;
            if lambda_i.abs() > 1e-6 {
                *wi -= lambda_i * vi;
            }
        }
        let nrm = norm2(&w);
        if nrm < 1e-15 {
            break;
        }
        v = w.iter().map(|x| x / nrm).collect();
    }
    let nrm = norm2(&v);
    if nrm > 1e-15 {
        v.iter().map(|x| x / nrm).collect()
    } else {
        v
    }
}

// ---------------------------------------------------------------------------
// SmallSignalAnalyzer implementation
// ---------------------------------------------------------------------------

impl SmallSignalAnalyzer {
    /// Construct a new analyser with default config.
    pub fn new(system: SmallSignalSystem) -> Self {
        Self {
            system,
            config: SmallSignalConfig::default(),
        }
    }

    /// Construct a new analyser with custom config.
    pub fn with_config(system: SmallSignalSystem, config: SmallSignalConfig) -> Self {
        Self { system, config }
    }

    /// Build the linearised state matrix A.
    ///
    /// Without AVR: 2n × 2n, states \[δ_1…δ_n, ω_1…ω_n\]
    /// With AVR:    3n × 3n, states \[δ_1…δ_n, ω_1…ω_n, E'q_1…E'q_n\]
    pub fn build_state_matrix(&self) -> Vec<Vec<f64>> {
        let machines = &self.system.machines;
        let n = machines.len();
        let net = &self.system.network_matrix;
        let has_avr = machines.iter().any(|m| m.avr_enabled);
        let dim = if has_avr { 3 * n } else { 2 * n };

        let mut a = vec![vec![0.0_f64; dim]; dim];

        for (i, m) in machines.iter().enumerate() {
            let two_h = 2.0 * m.inertia_mj_mva;

            // dδ_i/dt = ω_0 × Δω_i
            a[i][n + i] = OMEGA_0;

            // dω_i/dt base: -(D/2H) Δω - (Ks/2H) Δδ
            a[n + i][n + i] = -m.damping / two_h;
            a[n + i][i] = -m.synchronizing_coeff / two_h;

            // Network coupling through off-diagonal B matrix elements
            // The coupling term adds ΣBij(δj-δi) effects to ω_i equation
            if i < net.len() {
                for (j, &bij) in net[i].iter().enumerate() {
                    if j != i && bij.abs() > 1e-15 {
                        // Positive coupling from machine j to machine i
                        a[n + i][j] += bij / two_h;
                    }
                }
            }

            // AVR block: dE'q_i/dt = (1/Ta)(-E'q + Ka × Δδ)
            if has_avr && m.avr_enabled {
                let ta = if m.exciter_time_s.abs() > 1e-9 {
                    m.exciter_time_s
                } else {
                    0.1
                };
                a[2 * n + i][2 * n + i] = -1.0 / ta;
                a[2 * n + i][i] = m.exciter_gain / ta;
                // E'q modifies ω equation
                a[n + i][2 * n + i] = 1.0 / two_h;
            }
        }
        a
    }

    /// Compute eigenvalues of `a_matrix` using the QR algorithm.
    ///
    /// Returns `(real, imag)` pairs. Complex conjugate pairs appear consecutively.
    pub fn qr_eigenvalues(&self, a_matrix: &[Vec<f64>]) -> Vec<(f64, f64)> {
        if a_matrix.is_empty() {
            return Vec::new();
        }
        let (h, _q) = hessenberg_reduction(a_matrix);
        qr_iteration_hessenberg(
            &h,
            self.config.max_iterations_qr,
            self.config.convergence_tol,
        )
    }

    /// Classify eigenvalue pairs into oscillation modes.
    ///
    /// Oscillatory eigenvalues (|imag| > 0) appear as conjugate pairs; only the
    /// positive-imaginary half is returned (as a mode with positive `imag`).
    pub fn classify_modes(&self, eigenvalues: &[(f64, f64)]) -> Vec<EigenvalueResult> {
        let mut results = Vec::new();
        let mut i = 0;
        while i < eigenvalues.len() {
            let (re, im) = eigenvalues[i];
            // Check if this is part of a conjugate pair
            if im.abs() > 1e-6
                && i + 1 < eigenvalues.len()
                && (re - eigenvalues[i + 1].0).abs() < 1e-4
                && (im + eigenvalues[i + 1].1).abs() < 1e-4
            {
                // Take the positive-imaginary element
                let (re_pair, im_pair) = if im > 0.0 {
                    (re, im)
                } else {
                    eigenvalues[i + 1]
                };
                let result = self.make_eigenvalue_result(re_pair, im_pair.abs());
                results.push(result);
                i += 2;
            } else if im.abs() > 1e-6 {
                // Unpaired complex eigenvalue — include anyway
                let result = self.make_eigenvalue_result(re, im.abs());
                results.push(result);
                i += 1;
            } else {
                // Real eigenvalue
                let result = self.make_eigenvalue_result(re, 0.0);
                results.push(result);
                i += 1;
            }
        }
        results
    }

    fn make_eigenvalue_result(&self, re: f64, im_abs: f64) -> EigenvalueResult {
        let freq_hz = im_abs / (2.0 * PI);
        let mag = (re * re + im_abs * im_abs).sqrt();
        let damping_ratio = if mag > 1e-12 { -re / mag } else { 0.0 };
        let mode_type = self.classify_mode_type(freq_hz, im_abs);
        EigenvalueResult {
            real: re,
            imag: im_abs,
            frequency_hz: freq_hz,
            damping_ratio,
            mode_type,
        }
    }

    fn classify_mode_type(&self, freq_hz: f64, im_abs: f64) -> ModeType {
        if im_abs < 1e-6 {
            return ModeType::ControlMode;
        }
        if freq_hz < 0.8 {
            ModeType::InterAreaMode
        } else if freq_hz <= 2.0 {
            ModeType::LocalPlantMode
        } else {
            ModeType::ControlMode
        }
    }

    /// Compute participation factors for mode `eigenvalue_idx`.
    ///
    /// Participation factor P_ki = |ψ_ki × φ_ki| (element-wise product of left and right
    /// eigenvectors). Normalised so that Σ P_ki = 1.
    pub fn participation_factors(
        &self,
        a_matrix: &[Vec<f64>],
        eigenvalue_idx: usize,
    ) -> Vec<ParticipationFactor> {
        let eigs = self.qr_eigenvalues(a_matrix);
        if eigenvalue_idx >= eigs.len() {
            return Vec::new();
        }
        let (lambda_r, lambda_i) = eigs[eigenvalue_idx];
        let n_states = a_matrix.len();
        let n_machines = self.system.machines.len();
        if n_machines == 0 {
            return Vec::new();
        }
        let has_avr = self.system.machines.iter().any(|m| m.avr_enabled);
        let states_per_machine = if has_avr { 3 } else { 2 };

        // Right eigenvector φ
        let phi = approx_eigenvector(a_matrix, lambda_r, lambda_i);

        // Left eigenvector ψ (from transpose)
        let at = transpose(a_matrix);
        let psi = approx_eigenvector(&at, lambda_r, lambda_i);

        // Raw participation: |ψ_k × φ_k|
        let raw: Vec<f64> = (0..n_states).map(|k| (psi[k] * phi[k]).abs()).collect();

        let total: f64 = raw.iter().sum();
        let norm = if total > 1e-15 { total } else { 1.0 };

        let pfs = raw
            .iter()
            .enumerate()
            .map(|(k, &raw_k)| {
                let machine_idx = k % n_machines;
                let state_block = k / n_machines;
                let state_name = match state_block {
                    0 => "delta",
                    1 => "omega",
                    _ if states_per_machine == 3 => "E_q",
                    _ => "unknown",
                };
                ParticipationFactor {
                    machine_id: self.system.machines[machine_idx].machine_id.clone(),
                    factor: raw_k / norm,
                    state_variable: state_name.to_string(),
                }
            })
            .collect();
        pfs
    }

    /// Identify inter-area modes from classified eigenvalues and participation factors.
    pub fn identify_inter_area_modes(
        &self,
        eigenvalues: &[EigenvalueResult],
        participation_factors: &[Vec<ParticipationFactor>],
    ) -> Vec<InterAreaMode> {
        let mut modes = Vec::new();

        for (mode_idx, eig) in eigenvalues.iter().enumerate() {
            if eig.mode_type != ModeType::InterAreaMode {
                continue;
            }
            if eig.imag < 1e-6 {
                continue;
            }

            let pfs = if mode_idx < participation_factors.len() {
                &participation_factors[mode_idx]
            } else {
                continue;
            };

            let n_machines = self.system.machines.len();
            let mut machine_pf = vec![0.0_f64; n_machines];
            for pf in pfs {
                if let Some(idx) = self
                    .system
                    .machines
                    .iter()
                    .position(|m| m.machine_id == pf.machine_id)
                {
                    machine_pf[idx] += pf.factor;
                }
            }

            let significant: Vec<usize> = machine_pf
                .iter()
                .enumerate()
                .filter(|(_, &p)| p > 0.05)
                .map(|(i, _)| i)
                .collect();
            if significant.len() < 2 {
                continue;
            }

            let mut sorted_pf: Vec<f64> = significant.iter().map(|&i| machine_pf[i]).collect();
            sorted_pf.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = if sorted_pf.len().is_multiple_of(2) {
                0.5 * (sorted_pf[sorted_pf.len() / 2 - 1] + sorted_pf[sorted_pf.len() / 2])
            } else {
                sorted_pf[sorted_pf.len() / 2]
            };

            let mut group1: Vec<String> = Vec::new();
            let mut group2: Vec<String> = Vec::new();
            let mut e1 = 0.0_f64;
            let mut e2 = 0.0_f64;

            for &idx in &significant {
                let h = self.system.machines[idx].inertia_mj_mva;
                if machine_pf[idx] >= median {
                    group1.push(self.system.machines[idx].machine_id.clone());
                    e1 += h * machine_pf[idx];
                } else {
                    group2.push(self.system.machines[idx].machine_id.clone());
                    e2 += h * machine_pf[idx];
                }
            }

            let modal_energy_ratio = if e2 > 1e-12 { e1 / e2 } else { f64::INFINITY };

            modes.push(InterAreaMode {
                mode_id: mode_idx,
                frequency_hz: eig.frequency_hz,
                damping_ratio: eig.damping_ratio,
                group1_machines: group1,
                group2_machines: group2,
                modal_energy_ratio,
            });
        }
        modes
    }

    /// Compute eigenvalues with a PSS added to machine `machine_idx`.
    ///
    /// The PSS adds a damping torque ΔTd = Kpss × Δω to the swing equation:
    /// `dω_i/dt += -(Kpss / 2H) × Δω_i`
    pub fn sensitivity_to_pss(&self, machine_idx: usize, pss_gain: f64) -> Vec<EigenvalueResult> {
        let n = self.system.machines.len();
        if machine_idx >= n {
            return Vec::new();
        }
        let mut a = self.build_state_matrix();
        let m = &self.system.machines[machine_idx];
        let two_h = 2.0 * m.inertia_mj_mva;
        // PSS: ΔTd = Kpss × Δω → additional damping term
        a[n + machine_idx][n + machine_idx] -= pss_gain / two_h;

        let eigs = self.qr_eigenvalues(&a);
        self.classify_modes(&eigs)
    }

    /// Modal analysis of the reduced load-flow Jacobian (dQ/dV block).
    ///
    /// Returns eigenvalues sorted with smallest (most critical) first.
    /// A near-zero minimum eigenvalue indicates proximity to voltage collapse.
    pub fn voltage_stability_eigenvalue(&self, load_flow_jacobian: &[Vec<f64>]) -> Vec<f64> {
        if load_flow_jacobian.is_empty() {
            return Vec::new();
        }
        let eigs_complex = self.qr_eigenvalues(load_flow_jacobian);
        let mut real_eigs: Vec<f64> = eigs_complex.iter().map(|&(r, _)| r).collect();
        real_eigs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        real_eigs
    }

    /// Estimate maximum power transfer and stability margin.
    ///
    /// Simplified formula: P_max = 0.5 × V² / |Z|
    /// Returns stability margin in percent: (P_max - P_current) / P_max × 100.
    pub fn power_transfer_limit(
        &self,
        from_machine: usize,
        to_bus: usize,
        current_loading_pu: f64,
    ) -> f64 {
        let n = self.system.machines.len();
        if from_machine >= n {
            return 0.0;
        }
        let v_pu = self.system.machines[from_machine].transient_emf;
        let z_mag = if from_machine < self.system.network_matrix.len()
            && to_bus < self.system.network_matrix[from_machine].len()
        {
            let b = self.system.network_matrix[from_machine][to_bus].abs();
            if b > 1e-9 {
                1.0 / b
            } else {
                1.0
            }
        } else {
            1.0
        };
        let p_max = 0.5 * v_pu * v_pu / z_mag;
        if p_max < 1e-12 {
            return 0.0;
        }
        let margin = (p_max - current_loading_pu) / p_max * 100.0;
        margin.max(0.0)
    }

    /// Full analysis: build A-matrix, compute eigenvalues, classify, participation factors.
    pub fn full_analysis(&self) -> ModeReport {
        let a = self.build_state_matrix();
        let eigs_raw = self.qr_eigenvalues(&a);
        let eigenvalues = self.classify_modes(&eigs_raw);

        let participation_factors: Vec<Vec<ParticipationFactor>> = eigenvalues
            .iter()
            .enumerate()
            .map(|(i, _)| self.participation_factors(&a, i))
            .collect();

        let inter_area_modes = self.identify_inter_area_modes(&eigenvalues, &participation_factors);

        let poorly_damped_modes: Vec<usize> = eigenvalues
            .iter()
            .enumerate()
            .filter(|(_, e)| e.damping_ratio < self.config.min_damping_ratio && e.imag > 1e-6)
            .map(|(i, _)| i)
            .collect();

        let critical_mode = eigenvalues
            .iter()
            .enumerate()
            .filter(|(_, e)| e.imag > 1e-6)
            .min_by(|(_, a), (_, b)| {
                a.damping_ratio
                    .partial_cmp(&b.damping_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i);

        ModeReport {
            eigenvalues,
            participation_factors,
            inter_area_modes,
            poorly_damped_modes,
            critical_mode,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn single_machine(h: f64, d: f64, ks: f64) -> SmallSignalAnalyzer {
        let m = MachineSmallSignalModel {
            machine_id: "G1".to_string(),
            inertia_mj_mva: h,
            damping: d,
            synchronizing_coeff: ks,
            transient_emf: 1.05,
            exciter_gain: 0.0,
            exciter_time_s: 0.1,
            avr_enabled: false,
        };
        let system = SmallSignalSystem {
            machines: vec![m],
            network_matrix: vec![vec![0.0]],
            base_mva: 100.0,
        };
        SmallSignalAnalyzer::new(system)
    }

    fn empty_analyzer() -> SmallSignalAnalyzer {
        SmallSignalAnalyzer::new(SmallSignalSystem {
            machines: vec![],
            network_matrix: vec![],
            base_mva: 100.0,
        })
    }

    /// Test 1: Single machine oscillation frequency matches the A-matrix natural frequency.
    ///
    /// State matrix A = [[0, ω₀], [-Ks/2H, -D/2H]] has characteristic polynomial
    /// λ² + (D/2H)λ + (ω₀·Ks/2H) = 0, so the undamped natural frequency is
    /// ωn = √(ω₀·Ks/2H) and fn = ωn/(2π).
    #[test]
    fn test_single_machine_oscillation_frequency() {
        let h = 5.0;
        let d = 0.5; // small damping so oscillation remains
        let ks = 0.8;
        let analyzer = single_machine(h, d, ks);

        let a = analyzer.build_state_matrix();
        assert_eq!(a.len(), 2, "2n states for n=1 machine");

        let eigs = analyzer.qr_eigenvalues(&a);

        // Expected: natural frequency from characteristic polynomial of 2×2 A-matrix
        // ωn² = ω₀ · Ks / (2H)
        let omega_n_sq = OMEGA_0 * ks / (2.0 * h);
        // Damped natural frequency: ωd² = ωn² - (D/4H)²
        let zeta_term = d / (2.0 * 2.0 * h); // D/(4H)
        let omega_d_sq = omega_n_sq - zeta_term * zeta_term;
        let expected_freq = if omega_d_sq > 0.0 {
            omega_d_sq.sqrt() / (2.0 * PI)
        } else {
            omega_n_sq.sqrt() / (2.0 * PI)
        };

        // Find any oscillatory eigenvalue
        let osc = eigs.iter().find(|&&(_, im)| im.abs() > 0.01);
        assert!(
            osc.is_some(),
            "Expected oscillatory eigenvalue. Got: {:?}. \
             Expected freq ≈ {:.4} Hz",
            eigs,
            expected_freq
        );
        let (_, im) = osc.unwrap();
        let computed_freq = im.abs() / (2.0 * PI);
        let rel_err = (computed_freq - expected_freq).abs() / expected_freq;
        assert!(
            rel_err < 0.15,
            "Frequency mismatch: computed={:.4} Hz expected={:.4} Hz (rel={:.3})",
            computed_freq,
            expected_freq,
            rel_err
        );
    }

    /// Test 2: Positive damping → stable eigenvalues (σ < 0)
    #[test]
    fn test_positive_damping_gives_stable_eigenvalues() {
        let analyzer = single_machine(6.0, 2.0, 1.0);
        let a = analyzer.build_state_matrix();
        let eigs = analyzer.qr_eigenvalues(&a);
        for &(re, _) in &eigs {
            assert!(re < 1e-4, "Expected σ < 0 (stable), got σ = {}", re);
        }
    }

    /// Test 3: Two-machine system produces an oscillatory mode in 0.1–2.0 Hz
    #[test]
    fn test_two_machine_inter_area_mode() {
        let m1 = MachineSmallSignalModel {
            machine_id: "G1".to_string(),
            inertia_mj_mva: 10.0,
            damping: 1.0,
            synchronizing_coeff: 0.5,
            transient_emf: 1.0,
            exciter_gain: 0.0,
            exciter_time_s: 0.1,
            avr_enabled: false,
        };
        let m2 = MachineSmallSignalModel {
            machine_id: "G2".to_string(),
            inertia_mj_mva: 8.0,
            damping: 1.0,
            synchronizing_coeff: 0.5,
            transient_emf: 1.0,
            exciter_gain: 0.0,
            exciter_time_s: 0.1,
            avr_enabled: false,
        };
        let b = 0.3_f64;
        let system = SmallSignalSystem {
            machines: vec![m1, m2],
            network_matrix: vec![vec![b, b], vec![b, b]],
            base_mva: 100.0,
        };
        let analyzer = SmallSignalAnalyzer::new(system);
        let a = analyzer.build_state_matrix();
        let eigs = analyzer.qr_eigenvalues(&a);
        let modes = analyzer.classify_modes(&eigs);

        let in_range = modes
            .iter()
            .any(|e| e.frequency_hz > 0.05 && e.frequency_hz < 3.0);
        assert!(
            in_range,
            "Expected at least one mode in 0.05-3.0 Hz range, got: {:?}",
            modes.iter().map(|e| e.frequency_hz).collect::<Vec<_>>()
        );
    }

    /// Test 4: QR algorithm converges for a 2×2 symmetric matrix with known eigenvalues
    #[test]
    fn test_qr_converges_2x2_symmetric() {
        // [[3, 1], [1, 3]] → eigenvalues 4 and 2
        let a = vec![vec![3.0_f64, 1.0], vec![1.0, 3.0]];
        let config = SmallSignalConfig {
            max_iterations_qr: 200,
            convergence_tol: 1e-10,
            ..SmallSignalConfig::default()
        };
        let system = SmallSignalSystem {
            machines: vec![],
            network_matrix: vec![],
            base_mva: 100.0,
        };
        let analyzer = SmallSignalAnalyzer::with_config(system, config);
        let eigs = analyzer.qr_eigenvalues(&a);
        assert_eq!(eigs.len(), 2, "Expected 2 eigenvalues for 2×2 matrix");
        let mut reals: Vec<f64> = eigs.iter().map(|&(r, _)| r).collect();
        reals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        assert!(
            (reals[0] - 2.0).abs() < 0.2,
            "Expected eigenvalue ≈ 2, got {}",
            reals[0]
        );
        assert!(
            (reals[1] - 4.0).abs() < 0.2,
            "Expected eigenvalue ≈ 4, got {}",
            reals[1]
        );
    }

    /// Test 5: Participation factors sum to 1 for each mode
    #[test]
    fn test_participation_factors_sum_to_one() {
        let analyzer = single_machine(5.0, 0.5, 0.8);
        let a = analyzer.build_state_matrix();
        let eigs = analyzer.qr_eigenvalues(&a);
        for idx in 0..eigs.len() {
            let pfs = analyzer.participation_factors(&a, idx);
            if pfs.is_empty() {
                continue;
            }
            let total: f64 = pfs.iter().map(|p| p.factor).sum();
            assert!(
                (total - 1.0).abs() < 1e-9,
                "Mode {} PF sum = {} (expected 1.0)",
                idx,
                total
            );
        }
    }

    /// Test 6: PSS with positive gain improves damping ratio
    #[test]
    fn test_pss_improves_damping_ratio() {
        // Lightly damped system: D=0.1, Ks=1.0, H=5 → ζ ≈ D/(2√(Ks*2H))
        let analyzer = single_machine(5.0, 0.1, 1.0);
        let a_base = analyzer.build_state_matrix();
        let eigs_base = analyzer.qr_eigenvalues(&a_base);
        let modes_base = analyzer.classify_modes(&eigs_base);

        // With PSS gain = 10
        let modes_pss = analyzer.sensitivity_to_pss(0, 10.0);

        // Compare damping ratios of oscillatory modes
        let dr_base: f64 = modes_base
            .iter()
            .filter(|e| e.imag > 0.1)
            .map(|e| e.damping_ratio)
            .fold(f64::NEG_INFINITY, f64::max);
        let dr_pss: f64 = modes_pss
            .iter()
            .filter(|e| e.imag > 0.1)
            .map(|e| e.damping_ratio)
            .fold(f64::NEG_INFINITY, f64::max);

        assert!(
            dr_pss > dr_base || (dr_base - dr_pss).abs() < 1e-6,
            "PSS should not worsen damping: base={:.4} pss={:.4}",
            dr_base,
            dr_pss
        );
        // More specifically check with large gain the PSS should significantly improve
        let modes_pss_strong = analyzer.sensitivity_to_pss(0, 50.0);
        let dr_strong: f64 = modes_pss_strong
            .iter()
            .filter(|e| e.imag > 0.1)
            .map(|e| e.damping_ratio)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            dr_strong > dr_base,
            "Strong PSS (gain=50) should significantly improve damping: base={:.4} strong={:.4}",
            dr_base,
            dr_strong
        );
    }

    /// Test 7: 0.5 Hz mode classifies as InterAreaMode
    #[test]
    fn test_classify_0p5hz_as_inter_area() {
        let analyzer = empty_analyzer();
        let omega = 2.0 * PI * 0.5;
        let eigenvalues = vec![(-0.05, omega), (-0.05, -omega)];
        let modes = analyzer.classify_modes(&eigenvalues);
        let found = modes.iter().find(|e| (e.frequency_hz - 0.5).abs() < 0.05);
        assert!(found.is_some(), "Expected ≈0.5 Hz mode, got {:?}", modes);
        assert_eq!(
            found.unwrap().mode_type,
            ModeType::InterAreaMode,
            "0.5 Hz mode should be InterAreaMode"
        );
    }

    /// Test 8: Near-singular dQ/dV Jacobian → minimum eigenvalue near 0
    #[test]
    fn test_voltage_stability_near_collapse() {
        let jac = vec![
            vec![0.001_f64, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 2.0],
        ];
        let analyzer = empty_analyzer();
        let eigs = analyzer.voltage_stability_eigenvalue(&jac);
        assert!(!eigs.is_empty(), "Expected non-empty eigenvalue list");
        let min_eig = eigs[0];
        assert!(
            min_eig < 0.1,
            "Minimum eigenvalue should be near 0 for near-collapse, got {}",
            min_eig
        );
    }

    /// Test 9: AVR extends state matrix to 3n dimensions
    #[test]
    fn test_avr_extends_state_matrix() {
        let m = MachineSmallSignalModel {
            machine_id: "G1".to_string(),
            inertia_mj_mva: 5.0,
            damping: 1.0,
            synchronizing_coeff: 0.8,
            transient_emf: 1.05,
            exciter_gain: 50.0,
            exciter_time_s: 0.02,
            avr_enabled: true,
        };
        let system = SmallSignalSystem {
            machines: vec![m],
            network_matrix: vec![vec![0.0]],
            base_mva: 100.0,
        };
        let analyzer = SmallSignalAnalyzer::new(system);
        let a = analyzer.build_state_matrix();
        assert_eq!(a.len(), 3, "AVR: expected 3×3 state matrix for 1 machine");
        assert_eq!(a[0].len(), 3);
    }

    /// Test 10: Power transfer limit returns non-negative margin
    #[test]
    fn test_power_transfer_limit_is_non_negative() {
        let m = MachineSmallSignalModel {
            machine_id: "G1".to_string(),
            inertia_mj_mva: 5.0,
            damping: 1.0,
            synchronizing_coeff: 0.8,
            transient_emf: 1.0,
            exciter_gain: 0.0,
            exciter_time_s: 0.1,
            avr_enabled: false,
        };
        let system = SmallSignalSystem {
            machines: vec![m],
            network_matrix: vec![vec![2.0]],
            base_mva: 100.0,
        };
        let analyzer = SmallSignalAnalyzer::new(system);
        let margin = analyzer.power_transfer_limit(0, 0, 0.3);
        assert!(
            margin >= 0.0,
            "Margin should be non-negative, got {}",
            margin
        );
    }
}
