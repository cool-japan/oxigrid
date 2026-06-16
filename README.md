# OxiGrid

[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/cool-japan/oxigrid)
[![Tests](https://img.shields.io/badge/tests-6179%20passing-brightgreen)](https://github.com/cool-japan/oxigrid)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021%20%E2%80%A2%20MSRV%201.75-orange)](https://www.rust-lang.org)
[![COOLJAPAN](https://img.shields.io/badge/COOLJAPAN-ecosystem-blue)](https://github.com/cool-japan)

**OxiGrid** is a pure-Rust electrical power systems simulation and optimisation library. It provides
production-grade implementations of AC/DC power flow, transient and small-signal stability analysis,
battery electrochemical and thermal modelling, renewable energy integration, optimal power flow,
harmonic analysis, and protection system design — all without any C or Fortran dependencies.

---

## Overview

OxiGrid is part of the **COOLJAPAN ecosystem** — a collection of high-performance, pure-Rust
scientific and engineering libraries maintained by COOLJAPAN OU (Team Kitasan). The library is
designed to be a drop-in computational back-end for power systems tools requiring numerical
reliability, embedded deployability, and first-class Rust ergonomics.

**Design principles:**

- **Pure Rust** — no C/Fortran in the default build; all linear algebra via
  [oxiblas-lapack](https://github.com/cool-japan/oxiblas) and
  [oxiblas-sparse](https://github.com/cool-japan/oxiblas), FFT via
  [OxiFFT](https://github.com/cool-japan/oxifft)
- **Feature-gated** — compile only what you need; the full library adds zero overhead for unused
  subsystems
- **Numerically rigorous** — IEEE test case validated (14-bus, 30-bus, 57-bus, 118-bus, 300-bus),
  sparse Jacobian with auto-selected LU factorisation
- **`no_std` friendly** — units and ECM modules build without the standard library

---

## Features

### Power Flow (`powerflow`)

- **Newton-Raphson** AC power flow with sparse Jacobian (oxiblas-sparse) and automatic dense/sparse
  LU switching
- **Fast Decoupled Load Flow** (Stott & Alsac 1974) — decoupled B′/B″ matrices for large networks
- **DC Approximation** — linear B′-matrix formulation for fast contingency screening
- **Holomorphic Embedding Method** (HEM) — power series embedding, non-iterative convergence
- **Continuation Power Flow** — voltage stability boundary tracing with arc-length
  parameterisation
- **Branch flow computation** — π-model AC and DC branch P/Q flows, total system losses
- **State estimation** — AC weighted least-squares (WLS) with chi-squared bad-data detection
- **EKF dynamic state estimation** — Extended Kalman Filter for dynamic SE + oscillation detector
- **Three-phase unbalanced power flow** — Newton-Raphson for distribution networks
- **Probabilistic power flow** — Monte Carlo load uncertainty propagation
- **SIMD kernels** — AVX2 inner-loop acceleration (behind `simd` feature)
- **DC Power Flow** — full sparse DC power flow with B-matrix, LU factorisation, sensitivity matrix
- **Harmonic Power Flow** — harmonic-coupled power flow for harmonic interaction studies
- **Stochastic Load Flow** — Monte Carlo AC load flow with configurable uncertainty distributions
- Q-limit enforcement for PV buses (automatic PV→PQ switching)
- Warm-start Newton-Raphson for sequential time-series solving

### Transient & Small-Signal Stability (`stability`)

- **Transient stability** — swing equation, adaptive RK45 time-domain simulation with event queue,
  SMIB fault-on/fault-cleared trajectories, Critical Clearing Time (CCT) computation
- **Multi-machine** — full network-reduced admittance matrix, coupled generator dynamics
- **Small-signal** — multi-machine state-space A-matrix, Schur eigenvalue decomposition,
  inter-area and local oscillation mode identification
- **Voltage stability** — P-V and Q-V curve tracing, L-index, FVSI, N-1 voltage stability
  assessment, modal analysis
- **Generator models** — classical, detailed 4th-order d-q axis, IEEE Type-1 AVR, TGOV1 governor,
  PSS (power system stabiliser)
- **AGC** — multi-area automatic generation control, governor droop, FCR assessment, HVDC frequency
  support
- **Restoration** — black-start planning, restoration sequencer, SAIDI/SAIFI/ENS reliability indices
- **AVR** — IEEE Type I/II/III automatic voltage regulator with anti-windup, coupled into transient
  stability simulation
- **PSS Tuner** — PSS parameter optimisation via residue method and frequency-domain criteria
- **Load modelling** — ZIP, motor, composite load models

### Battery Modelling (`battery`)

- **Equivalent Circuit Models** — Rint, 1RC (Thevenin), 2RC with tabulated OCV-SoC curves
- **SoC Estimation** — Coulomb counting, Extended Kalman Filter (EKF), Unscented Kalman Filter
  (UKF)
- **Thermal modelling** — lumped single-node and 1D finite-difference thermal models; Joule heating,
  convective and conductive cooling
- **Aging** — SEI growth (calendar + cycling), lithium plating, capacity and power fade maps
- **P2D / DFN model** — Single Particle Model electrochemical solver with electrolyte dynamics and
  separator
- **BMS** — fault detection, safety monitoring, charging scheduler (CC/CV, multi-stage)
- **Pack-level** — series/parallel cell assembly, passive cell balancing, BatteryCell/Module/Pack
  hierarchy
- **State of Power** — StatePowerEstimator (binary-search SoP), CapacityFadeEstimator

### Renewable Energy (`renewable`)

- **Solar PV** — Spencer 1971 solar position, Liu & Jordan plane-of-array irradiance, single-diode
  5-parameter cell model, MPPT (Perturb & Observe, Incremental Conductance), CEC/Sandia inverter
  models, shading loss computation
- **Wind** — turbine power curve with Betz limit, Weibull annual energy production, Jensen and
  Frandsen wake models, regular-grid wind farm layout optimisation, offshore 15 MW turbine model,
  Larsen wake model, MTDC offshore integration
- **Forecasting** — persistence (naive + diurnal), AR/ARIMA/SARIMA time series, ensemble methods,
  probabilistic forecast intervals, neural-network bridge trait, conformal prediction, quantile
  regression
- **Grid codes** — LVRT/HVRT profiles (IEC 61400-21/ENTSO-E/NERC/BDEW), FCR/FRR/FFR frequency
  response, PQ capability diagram, ramp rate limits
- **Grid integration analysis** — hosting capacity, SCR/WSCR/ESCR, inertia/ROCOF assessment
- **Grid-forming inverter** — VSM swing equation, droop control, MicrogridSimulator
- **Grid-following inverter** — SRF-PLL, current controller, LCL filter RK4 state-space

### Optimisation (`optimize`)

- **DC-OPF** — lambda-iteration (economic dispatch) + LP formulation via OxiZ LP solver
- **AC-OPF** — SQP/penalty interior-point, reactive power dispatch (ORPD)
- **SCOPF** — N-1 security-constrained OPF with contingency enumeration, multi-period (SCOPF-MP)
- **Unit Commitment** — priority-list heuristic + MILP branch-and-bound (MILP UC)
- **Multi-period OPF** — ramp constraints, storage inter-temporal coupling
- **Microgrid EMS** — rule-based and MPC energy management, islanding detection, peer-to-peer
  energy market
- **EV charging** — SmartCharger (TOU/V2G DP/frequency regulation), fleet valley-filling/peak-
  shaving, V2G aggregator with flexibility envelope and grid services
- **Hydrogen / P2G** — electrolyzer (PEM/ALK/SOEC + Butler-Volmer IV), hydrogen tank (5 types),
  fuel cell CHP, P2G dispatcher (4 modes)
- **Storage Arbitrage** — price-based battery dispatch scheduling, battery sizing optimisation,
  stochastic DP backward induction + ADP
- **Market clearing** — DAM/RTM/ancillary service clearing, LMP computation, DSO flexibility
  market (PTDF-filtered merit-order), demand response programs (price elasticity, VOLL)
- **Expansion Planning** — robust TEP (Benders decomposition), generation and network expansion
- **Reliability** — N-1 contingency reliability indices
- **Carbon OPF** — carbon-constrained DC-OPF with hard CO₂ cap, Green LMP, and Pareto-front sweep
- **N-1 SCOPF** — N-1 security-constrained OPF with corrective action dispatch
- **Stochastic UC** — scenario-tree stochastic unit commitment with expected-cost minimisation
- **Carbon Market** — EU ETS-style allowance auction, permit allocation, multi-year carbon plans
- **P2P Energy Market** — peer-to-peer trading with six clearing mechanisms
- **Multi-energy Hub** — EnergyHub/MesOptimizer coupling electricity, gas, heat, H₂ via converters

### Harmonics Analysis (`harmonics`)

- **THD / THVD** computation using OxiFFT (pure-Rust FFT)
- **Goertzel algorithm** for targeted harmonic order detection
- **IEEE 519-2022** voltage distortion compliance checking
- **IEC 61000-3-2** current limit evaluation
- **Passive filter design** — single-tuned and C-type high-pass RLC filters
- **Flicker** — Pst/Plt flicker severity estimation

### Protection System (`protection`)

- **Fault analysis** — Z-bus symmetrical component method; 3-phase (3LG), single-line-to-ground
  (SLG), line-to-line (LL), double-line-to-ground (DLG); DC offset factor; IEC 60909
- **Relay models** — IEC 60255 / IEEE C37.112 inverse-time overcurrent (IDMT), Mho distance relay
- **Protection coordination** — TCC curve coordination, coordination time interval (CTI)
  verification, advanced coordination
- **Differential protection** — transformer and line differential relay
- **Auto-recloser** — reclosing sequence logic
- **HIF Detection** — high-impedance fault detection (even-harmonic ratio, half-cycle asymmetry,
  incremental-energy) with Dempster-Shafer evidence fusion
- **Fault Current Limiter** — SFCL, resistive, and bridge-type FCL models
- **FLISR** — Fault Location, Isolation and Service Restoration for distribution automation
- **Zone Protection Coordination** — comprehensive relay grading with CTI verification

### Power Quality (`powerquality`)

- **PQ event classification** — IEEE 1159 taxonomy, PqEventClassifier
- **Indices** — THD, K-factor, crest factor, EN 50160/IEEE 519/IEC 61000-3-2/NERC TPL compliance
- **Sag/swell detection** — half-cycle RMS, ITIC/SEMI F47 curves
- **Waveform analysis** — time-frequency analysis, event characterisation

### Analytics (`analytics`)

- **Grid KPI Dashboard** — IEEE 1366 reliability metrics (SAIDI, SAIFI, CAIDI, ASAI, ENS), power
  quality (THD), and economic metrics (LCOE, NPV, IRR) via `GridKpiDashboard`
- **Grid Health Scorer** — component-level health scoring with configurable category weights and
  `GridHealthReport` aggregation
- **Carbon Accounting** — scope 1/2/3 CO₂e accounting, `CarbonBudgetTracker`, ETS tracking, grid
  emission intensity, ISO 14064-1 compliant reporting; `CarbonIntensityForecaster`
- **Energy Equity** — energy burden, affordability, and Gini-coefficient equity metrics
- **Operational Analytics** — time-series KPI trending with anomaly scoring and `OperationalDashboard`
- **Predictive Maintenance** — asset RUL estimation, `DegradationModel`, `HealthIndex`, maintenance
  schedule generation
- **Congestion Analytics** — real-time congestion and renewable performance KPI reporting

### Simulation (`simulation`)

- **Co-simulation Framework** — cyber-physical coupling: physical voltage dynamics, SCADA
  communication (latency, packet-loss), control layer, CUSUM-based cyber-attack detection; supports
  false-data injection, replay, DoS, and man-in-the-middle scenarios
- **Operator Training Simulator** — scenario-based training with automatic event injection, action
  grading, emergency procedure guidance, and competency scoring

### Security (`security`)

- **Anomaly Detection** — z-score, EWMA, and CUSUM detectors for grid measurement streams with
  `MeasurementCorrelationAnalyzer` for cross-sensor change detection
- **Data Integrity** — hash-chain audit trail for measurement integrity verification

### Wide-Area Monitoring (`io`/WAMS)

- **WAMS Analyzer** — GPS-synchronized PMU-based inter-area oscillation detection (AR Prony),
  frequency coherency clustering (K-means), L-index voltage stability proxy, alarm generation

### Grid Digital Twin (`digitaltwin`)

- **GridDigitalTwin** — DC WLS state estimation + NR power flow, SCADA/PMU telemetry ingestion
- **AlertEngine** — deduplication, severity classification, suppression logic
- **GridReplay** — what-if scenario simulation, KPI computation
- **Telemetry** — TelemetryFrame, ScadaMeasurement, PmuFrame ingestion pipeline

### Test Cases (`testcases`)

- **IEEE standard networks** — 14-bus, 30-bus, 57-bus, 118-bus, 300-bus exact data, RTS-96,
  PEGASE-89
- **Synthetic topologies** — Ring, Radial, Meshed, Geographic, SmallWorld, ScaleFree (6 types)
- **Benchmark suite** — BenchmarkScenario collection with reference solutions

### I/O and Formats (`io`)

- **MATPOWER** `.m` file import — baseMVA, bus, branch, gen sections; full round-trip export
- **IEEE Common Data Format** (IEEE CDF) parser
- **pandapower JSON** network import
- **CSV time-series** import/export — load profiles, generation schedules
- **PMU synchrophasor** processing, Prony analysis, Park transform
- **Serde** serialisation for all public structs (JSON, etc.)

### Units & Conversions (`units`)

- Newtype wrappers for `Voltage`, `Current`, `Power`, `ReactivePower`, `Impedance`,
  `Frequency`, `PerUnit`, `Temperature`, `Energy`, `Capacity`, `StateOfCharge`
- Arithmetic operators, `Display` with SI units, `PerUnit` conversion helpers
- `From` trait interoperability (`Voltage × Current → Power`, energy↔power)
- `no_std` compatible

---

## Quick Start

Add OxiGrid to `Cargo.toml`:

```toml
[dependencies]
oxigrid = "0.1.3"
```

To enable specific subsystems only:

```toml
[dependencies]
oxigrid = { version = "0.1.3", default-features = false, features = ["powerflow", "battery"] }
```

### Newton-Raphson Power Flow

```rust
use oxigrid::prelude::*;

fn main() -> Result<()> {
    // Load an IEEE 14-bus network from MATPOWER format
    let network = PowerNetwork::from_matpower("tests/data/ieee14.m")?;

    println!("Buses: {}, Branches: {}", network.bus_count(), network.branch_count());

    // Configure Newton-Raphson AC power flow
    let config = PowerFlowConfig {
        method: PowerFlowMethod::NewtonRaphson,
        max_iter: 50,
        tolerance: 1e-8,
        enforce_q_limits: false,
    };

    let result = network.solve_powerflow(&config)?;

    if result.converged {
        println!(
            "Converged in {} iterations (max mismatch: {:.2e} p.u.)",
            result.iterations, result.max_mismatch
        );
    }

    // Bus voltages
    for (i, (vm, va)) in result.voltage_magnitude.iter()
        .zip(result.voltage_angle.iter())
        .enumerate()
    {
        println!("Bus {:>3}: |V| = {:.4} p.u.  angle = {:>8.3} deg",
            i + 1, vm, va.to_degrees());
    }

    println!("System losses: {:.3} MW  {:.3} MVAr",
        result.total_p_loss_mw, result.total_q_loss_mvar);

    // Branch power flows
    for flow in result.branch_flows.iter().take(5) {
        println!("Branch {}->{}: P = {:.2} MW, Q = {:.2} MVAr",
            flow.from_bus, flow.to_bus, flow.p_from_mw, flow.q_from_mvar);
    }

    Ok(())
}
```

### DC Power Flow Approximation

```rust
use oxigrid::prelude::*;

fn main() -> Result<()> {
    let network = PowerNetwork::from_matpower("tests/data/ieee30.m")?;
    let config = PowerFlowConfig {
        method: PowerFlowMethod::DcApproximation,
        ..Default::default()
    };
    let result = network.solve_powerflow(&config)?;
    println!("DC power flow: {} buses, {} branches", network.bus_count(), network.branch_count());
    Ok(())
}
```

### Battery ECM Simulation

```rust
use oxigrid::prelude::*;
use oxigrid::battery::ecm::{TwoRcModel, TwoRcParams};
use oxigrid::battery::soc::CoulombCounter;
use oxigrid::units::{Current, StateOfCharge, Temperature};

fn main() -> Result<()> {
    let params = TwoRcParams::default(); // NMC defaults
    let model = TwoRcModel::new(params);
    let mut soc_estimator = CoulombCounter::new(
        StateOfCharge::new(0.8),
        100.0, // Ah capacity
    );

    let temp = Temperature::from_celsius(25.0);
    let soc = StateOfCharge::new(0.8);
    let current = Current(20.0); // 20 A discharge

    let vt = model.terminal_voltage(soc, current, temp);
    println!("Terminal voltage at SoC=0.8, I=20A, T=25°C: {}", vt);

    Ok(())
}
```

---

## Module Overview

| Module | Description |
|--------|-------------|
| `analytics` | Grid operations KPIs, congestion analysis, renewable metrics, demand analytics |
| `battery` | ECM, BMS, aging, thermal, pack, SoP/SoC estimation, P2D DFN model |
| `digitaltwin` | Grid digital twin, SCADA/PMU telemetry, alert engine, replay scenarios |
| `harmonics` | THD/spectrum analysis (OxiFFT), power quality, IEC/IEEE standards |
| `io` | CSV/MATPOWER export, time series, PMU synchrophasor, serialization |
| `monitoring` | Frequency monitoring, ROCOF relay, UFLS, nadir estimation, inertia estimation |
| `network` | Bus/branch models, admittance, topology (petgraph), FACTS, HVDC/MTDC, resilience, transformers, reconfiguration, voltage regulation |
| `optimize` | DC/AC/BESS OPF, ORPD, SCOPF, unit commitment (MILP B&B), market clearing (DAM/RTM/ancillary/LMP/DSO), MPC EMS, EV charging/fleet/V2G, hydrogen P2G, storage arbitrage/stochastic DP, demand response, microgrid advanced EMS, expansion planning |
| `planning` | Distribution expansion planning, asset condition assessment, RCM, DER integration, long-term strategy |
| `powerflow` | Newton-Raphson (warm-start), fast decoupled, DC, continuation (CPF), probabilistic, HEM, sparse LU, state estimation (AC WLS), unbalanced 3-phase, EKF dynamic SE, SIMD kernels |
| `powerquality` | PQ event classification, indices (THD/K-factor), sag/swell detection, waveform analysis, standards compliance (EN 50160/IEEE 519/IEC 61000-3-2/NERC TPL) |
| `protection` | Relay coordination, differential/distance protection, autorecloser, fault analysis (symmetric + asymmetric SLG/LL/DLG), IEC 60909, advanced coordination |
| `renewable` | Solar PV (irradiance/MPPT/shading/inverter), wind (turbine/wake/farm/spatial/offshore 15MW), forecasting (ARIMA/SARIMA/persistence/ensemble/NN bridge/probabilistic/conformal/quantile regression), grid codes (LVRT/HVRT/frequency response/PQ/ramp), grid integration analysis |
| `security` | Anomaly detection, cyber-physical attack modeling, NERC CIP checker, threat intelligence (MITRE ATT&CK for ICS), SCADA security assessment, incident response playbook |
| `simulation` | Simulation module |
| `stability` | Generator models (classical/detailed/AVR/governor/PSS), multi-machine, transient (RK45+event queue+CCT), small signal, modal, voltage stability (L-index/FVSI), AGC, restoration, load modeling (ZIP/motor/composite) |
| `testcases` | IEEE 14/30/57/118/300-bus, RTS-96, PEGASE-89, synthetic topologies, benchmark suite |
| `units` | Electrical, energy, thermal unit conversion |

---

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `std` | Standard library support | yes |
| `powerflow` | AC/DC power flow solvers, state estimation | yes |
| `stability` | Transient and small-signal stability analysis | yes |
| `battery` | Equivalent circuit models, BMS, SoC estimation | yes |
| `battery-p2d` | P2D DFN electrochemical model (requires `battery`) | yes |
| `renewable` | Solar PV, wind, forecasting, grid codes | yes |
| `optimize` | OPF, unit commitment, EV, hydrogen, market clearing | yes |
| `harmonics` | THD/spectrum analysis, filter design | yes |
| `protection` | Fault analysis, relay coordination | yes |
| `powerelectronics` | Power electronics models | yes |
| `forecast-ml` | ML-based forecasting bridge (requires `renewable`) | no |
| `io-matpower` | MATPOWER format I/O (requires `powerflow`) | no |
| `io-csv` | CSV time-series import/export | no |
| `io-oxirs` | OxiRS knowledge graph integration | no |
| `simd` | AVX2 SIMD kernels for Newton-Raphson inner loop | no |
| `parallel` | Rayon parallelism for Jacobian construction | no |

Disable the default feature set and opt-in selectively for minimal binary size:

```toml
oxigrid = { version = "0.1.3", default-features = false, features = ["powerflow"] }
```

Enable the full library including LP/MILP solver and SIMD acceleration:

```toml
oxigrid = { version = "0.1.3", features = ["simd", "parallel"] }
```

---

## Performance

Benchmarks are provided for standard IEEE test cases using
[Criterion](https://github.com/bheisler/criterion.rs). The following targets have been measured
on a modern desktop (Apple M-series); your results will vary by platform.

| Benchmark | Method | Target |
|-----------|--------|--------|
| IEEE 14-bus | Newton-Raphson | < 1 ms |
| IEEE 14-bus | DC Approximation | < 0.1 ms |
| IEEE 30-bus | Newton-Raphson | < 2 ms |
| IEEE 118-bus | Newton-Raphson | < 10 ms |
| IEEE 118-bus | DC Approximation | < 1 ms |
| IEEE 300-bus | Newton-Raphson | < 50 ms |

Run benchmarks locally:

```bash
cargo bench --features powerflow
```

The sparse LU auto-selector in `src/powerflow/sparse_lu.rs` chooses dense factorisation for
networks up to 200 buses and switches to the oxiblas-sparse CSR solver beyond that threshold,
keeping small-network overhead minimal while scaling to large systems.

Enable SIMD-accelerated Newton-Raphson for maximum throughput:

```bash
cargo bench --features full
```

---

## Architecture

OxiGrid sits atop a set of pure-Rust mathematical dependencies and exposes domain-specific
modules that can be composed freely:

```
┌─────────────────────────────────────────────────────────────────┐
│              COOLJAPAN Ecosystem Dependencies                    │
│                                                                  │
│  oxiblas-lapack   oxiblas-sparse   OxiFFT   OxiZ-theories       │
│  (dense linalg)   (sparse CSR LU) (pure FFT) (LP/MILP solver)   │
└──────────────┬────────────┬───────────┬──────────┬──────────────┘
               │            │           │          │
               ▼            ▼           ▼          ▼
┌─────────────────────────────────────────────────────────────────┐
│                          OxiGrid                                │
│                                                                  │
│  ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌──────────────────┐  │
│  │ network │  │powerflow │  │stability│  │     battery      │  │
│  │  Y-bus  │  │  NR/FDLF │  │transient│  │  ECM · P2D · BMS │  │
│  │topology │  │  DC/HEM  │  │modal/SS │  │  thermal · aging  │  │
│  │  FACTS  │  │ cont./SE │  │AGC/rest.│  │  SoC (EKF/UKF)   │  │
│  │  HVDC   │  │ unbal/EKF│  │generator│  │  pack · SoP      │  │
│  └─────────┘  └──────────┘  └─────────┘  └──────────────────┘  │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────────┐   │
│  │renewable │  │ optimize │  │harmonics │  │  protection   │   │
│  │solar/PV  │  │OPF/SCOPF │  │THD/flick.│  │fault/relay    │   │
│  │wind/offshr│ │MILP UC   │  │IEEE 519  │  │coordination   │   │
│  │grid codes│  │EV/H2/EMS │  │filters   │  │auto-recloser  │   │
│  │forecasting│ │market/DR │  └──────────┘  └───────────────┘   │
│  └──────────┘  └──────────┘                                     │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────────┐   │
│  │digitaltwin│ │powerquality│ │testcases │  │  units/io     │   │
│  │twin/alert │ │events/idx│  │IEEE/synth│  │  SI types     │   │
│  │replay/    │ │sag/swell │  │benchmark │  │  MATPOWER/CSV │   │
│  │telemetry  │ │standards │  └──────────┘  └───────────────┘   │
│  └──────────┘  └──────────┘                                     │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────────┐   │
│  │analytics │  │monitoring│  │ security │  │   planning    │   │
│  │KPI/cong. │  │freq/ROCOF│  │anomaly/  │  │TEP/asset/RCM  │   │
│  │renewable │  │UFLS/inert│  │NERC CIP  │  │DER/long-term  │   │
│  │demand    │  │nadir est.│  │SCADA sec.│  └───────────────┘   │
│  └──────────┘  └──────────┘  └──────────┘                      │
└─────────────────────────────────────────────────────────────────┘
```

### Linear Algebra Strategy

| Network size | Dense/sparse choice | Backend |
|---|---|---|
| ≤ 200 buses | Dense LU factorisation | `oxiblas-lapack` |
| > 200 buses | Sparse CSR LU | `oxiblas-sparse` |

The selection is automatic and transparent — `solve_auto()` in `src/powerflow/sparse_lu.rs`
inspects the matrix dimensions at runtime.

---

## Testing

OxiGrid ships 6,179 tests covering unit, integration, property-based, and benchmark scenarios.

```bash
# Run the full test suite (recommended: nextest for parallel execution)
cargo nextest run --all-features

# Standard cargo test
cargo test --all-features

# Run a specific module
cargo test --features powerflow powerflow

# Property-based tests (proptest)
cargo test --all-features proptest

# Build documentation (all features, no external deps required)
cargo doc --all-features --no-deps --open
```

Test data for IEEE standard networks is located in `tests/data/`:

| File | Buses | Branches | Source |
|------|-------|----------|--------|
| `ieee14.m` | 14 | 20 | IEEE 14-bus test case |
| `ieee30.m` | 30 | 41 | IEEE 30-bus test case |
| `ieee57.m` | 57 | 80 | IEEE 57-bus test case |
| `ieee118.m` | 118 | 186 | IEEE 118-bus test case |
| `ieee300.m` | 300 | 411 | IEEE 300-bus test case |

---

## Examples

Four runnable examples are provided in the `examples/` directory:

| Example | Feature | Description |
|---------|---------|-------------|
| `ieee14_powerflow` | `powerflow` | NR, DC, and FDLF on the IEEE 14-bus; branch flows and losses |
| `battery_cycling` | `battery` | CC/CV charge-discharge cycle with Coulomb counting and 1RC ECM |
| `microgrid_optimization` | `optimize`, `renewable` | Microgrid EMS dispatch with PV and battery storage |
| `renewable_forecast` | `renewable` | ARIMA solar generation forecast with persistence baseline |

```bash
cargo run --example ieee14_powerflow --features powerflow
cargo run --example battery_cycling --features battery
cargo run --example microgrid_optimization --features "optimize,renewable"
cargo run --example renewable_forecast --features renewable
```

---

## Project Statistics

Measured with `tokei` on the current codebase (2026-06-16):

| Language | Files | Code | Comments | Blanks |
|----------|-------|------|----------|--------|
| Rust | 483 | 302,247 | — | — |
| TOML | 2 | 94 | — | 10 |
| Markdown | 3 | — | 734 | 187 |
| **Total** | **488** | **305,108** | — | — |

- **Version**: 0.1.3
- **Tests**: 6,179 passing
- **Modules**: 22
- **Last Updated**: 2026-06-16

---

## Contributing

Contributions are welcome. Please ensure:

1. `cargo fmt --all` — code is formatted
2. `cargo clippy --all-features -- -D warnings` — no clippy warnings
3. `cargo nextest run --all-features` — all 6,179 tests pass
4. New public API items carry `///` doc comments
5. No `unwrap()` in production code paths — use `?` and `OxiGridError`
6. Feature-gate any new optional subsystems in `Cargo.toml` and `src/lib.rs`
7. Follow the Pure Rust policy — no C/Fortran in default features

---

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or
<https://www.apache.org/licenses/LICENSE-2.0>).

---

## Authors

**COOLJAPAN OU (Team Kitasan)**
<https://github.com/cool-japan>
