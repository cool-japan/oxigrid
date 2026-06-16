# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.3] - Unreleased

### Added

### Changed

### Fixed

## [0.1.2] - 2026-06-16

### Added

#### Analytics
- New `analytics` module with standard IEEE 1366 reliability KPIs (SAIDI, SAIFI, CAIDI, ASAI, ENS),
  power quality metrics (THD), and economic metrics (LCOE, NPV, IRR) via `GridKpiDashboard`
- `GridHealthScorer` — component-level health scoring by category with configurable weights and
  `GridHealthReport` aggregation
- `CarbonAccountant` — scope 1/2/3 CO₂e accounting with `CarbonBudgetTracker`, ETS scheme tracking,
  grid emission intensity (average and marginal), and ISO 14064-1 compliant reporting
- `CarbonIntensityForecaster` — fuel-mix-based marginal emission rate forecasting
- `EnergyEquityAnalyzer` — energy burden, affordability, and gini-coefficient equity metrics
- `OperationalAnalytics` — time-series KPI trending with anomaly scoring and `OperationalDashboard`
- `PredictiveMaintenance` — asset RUL estimation with `DegradationModel`, `HealthIndex`, and
  maintenance schedule generation
- `CongestionAnalyzer` and `OperationsReport` for real-time generation and renewable performance KPIs

#### Network
- `FlisrSystem` — FLISR (Fault Location, Isolation and Service Restoration) for distribution
  automation: fault indicator scanning, minimum-zone isolation, tie-switch restoration search
- `DynamicLineRating` — temperature/weather-driven thermal ampacity with IEEE 738 heat balance model
- `TheveninEquivalent` — network reduction to Thevenin impedance via iterative perturbation method
- `NetworkReduction` — Ward/REI equivalents and kron reduction for large system simplification
- `AdmittanceMatrix` — full Y-bus construction with tap/phase-shift transformer handling
- `CongestionManager` — line-flow congestion detection and redispatch cost computation
- `UpfcModel` — detailed UPFC power injection model with series and shunt compensation
- Resilience planning types (`ResiliencePlanningTypes`, `InfrastructureHardeningPlan`) for
  multi-hazard N-k hardening optimisation
- Voltage regulation type hierarchy (`VoltageRegulationConfig`, tap-changer / capacitor-bank control)
- MatPower format import/export enhancements (`src/network/formats/matpower.rs`)

#### Power Flow
- `DcPowerFlow` — full sparse DC power flow with B-matrix assembly, LU factorisation, and
  sensitivity-matrix computation (897 lines, `src/powerflow/dc_powerflow.rs`)
- `HarmonicPowerFlow` — harmonic-coupled power flow (fundamental + harmonic bus voltages) for
  network harmonic interaction studies
- `StochasticLoadFlow` — Monte Carlo AC load flow with configurable uncertainty distributions for
  renewable injection and load variability
- `UnbalancedContinuationPowerFlow` — predictor-corrector continuation for three-phase unbalanced
  systems with nose-point detection
- `PowerFlowJacobian` — standalone Jacobian builder with sparsity-pattern caching for use outside
  the NR solver inner loop
- Refactored `timeseries_sim` from a 1,994-line monolithic file into a well-structured module
  (`functions.rs`, `types.rs`, `types_4.rs`) for maintainability
- Enhanced `PowerFlowResult` with detailed line-loss, reactive-power, and convergence diagnostics

#### Optimization — OPF
- `CarbonOpfSolver` — carbon-constrained DC-OPF with hard CO₂ cap enforcement, augmented marginal
  cost (dual cost/emission weight), and Pareto-front sweep (`CarbonOpfResult`, `Green LMP`)
- `N1Scopf` — N-1 security-constrained OPF with post-contingency constraint enforcement and
  corrective action dispatch
- `SecurityOPF` — bus-level security OPF coupling voltage/thermal limits with contingency screening
- Expanded multi-period OPF with improved ramp-product modelling

#### Optimization — Dispatch
- `EconomicDispatch` — merit-order economic dispatch with incremental heat-rate curves
- `RampProductMarket` — ramp capability product market clearing (up/down ramp capacity, prices)
- `StochasticUnitCommitment` — scenario-tree stochastic UC with expected-cost minimisation

#### Optimization — Market
- `CarbonBudget` / `CarbonMarket` — EU ETS-style carbon allowance market with price dynamics,
  auction clearing, permit allocation methods (grandfathering, benchmarking, auction), and
  multi-year carbon plans; Pareto cost-vs-emissions analysis
- `P2pMarket` — peer-to-peer energy trading with six clearing mechanisms: double-sided auction,
  community micromarket, bilateral contract, blockchain ledger, virtual net-billing, and
  flexibility market; prosumer bid/offer matching with congestion-aware network constraints
- `RestorationSequenceOptimizer` — optimal switching sequence for post-fault service restoration
  subject to generation headroom and CLPU frequency constraints (340 lines)

#### Optimization — Multi-Energy & Microgrid
- `EnergyHub` / `MesOptimizer` — multi-energy system hub model coupling electricity, gas, heat,
  cooling, and hydrogen through converters (CHP, heat pump, boiler, electrolyser) and storage
  with greedy DP dispatch
- `MicrogridSizingOptimizer` — optimal DER sizing (PV + BESS + diesel) for islanded/grid-connected
  microgrids with LCOE and reliability constraints

#### Optimization — Expansion
- `StochasticTepV2` — enhanced stochastic transmission expansion planning v2 with improved
  scenario tree and Benders decomposition

#### Stability
- `BlackStartProcedure` — full black-start capability assessment: cranking-path discovery (BFS),
  step-by-step restoration plan respecting generation headroom and cold-load-pickup frequency
  constraints, and time-domain simulation tracking frequency nadir and voltage violations
- `AvrModel` — automatic voltage regulator (IEEE Type I/II/III) with anti-windup limiting, coupled
  into transient stability simulation
- `PssTuner` — PSS parameter optimisation using residue method and frequency-domain criteria

#### Protection
- `HifDetector` — high-impedance fault detection combining even-harmonic ratio, half-cycle
  asymmetry, and incremental-energy methods with Dempster-Shafer evidence fusion
- `FaultCurrentLimiter` — superconducting (SFCL), resistive, and bridge-type FCL models
- `ZoneProtectionCoordinator` — comprehensive zone protection coordination with 966-line full
  relay grading, CTI verification, and coordination report generation

#### Renewable Energy
- Enhanced solar submodule: `IrradianceModel` (direct/diffuse/reflected decomposition),
  `MpptController` (P&O and INC algorithms), `PvCellModel` (single-diode IV curve)
- `IntegrationStudy` — renewable integration study workflow (hosting capacity, fault-level
  impact, harmonic contribution, protection coordination)
- Grid codes: expanded `HvrtProfile` and `RampRateLimiter` for ENTSO-E RfG compliance

#### Simulation
- `CosimFramework` — cyber-physical co-simulation coupling physical voltage dynamics, SCADA
  communication (latency, packet-loss), control layer, and CUSUM-based cyber-attack detection;
  supports false-data injection, replay, DoS, and man-in-the-middle attack scenarios
- `OperatorTrainingSimulator` — scenario-based operator training with automatic event injection,
  action grading, emergency procedure guidance, and competency scoring
- Refactored `grid_ops` from a 1,994-line monolithic file into a structured module
  (`constants.rs`, `functions.rs`, `types.rs`, `types_4.rs`, `tests.rs`)

#### Security
- `GridAnomalyDetector` — z-score, EWMA, and CUSUM-based anomaly detection for grid measurement
  streams with `MeasurementCorrelationAnalyzer` for cross-sensor change detection
- `DataIntegrityChecker` — measurement integrity verification with hash-chain audit trail

#### Monitoring
- `WamsAnalyzer` — Wide-Area Monitoring System based on GPS-synchronized PMU phasors: angular
  stability index, inter-area oscillation detection (AR Prony), frequency coherency clustering
  (K-means), voltage stability L-index proxy, and alarm generation with severity classification
- `OscillationMonitor` types for structured inter-area mode tracking

#### Battery
- `BatteryAgingModel` — cycle-counting and calendar aging model with capacity-fade and
  resistance-growth estimation
- Battery ECM enhancements: L-BFGS parameter identification (`ecm/lbfgs.rs`), R-int and RC
  parameter structs with improved state-of-charge dependency

#### Harmonics
- `HarmonicPowerFlow` integration within harmonics module (`harmonics/harmonic_pf.rs`)
- Enhanced passive/active filter design (`harmonics/filter.rs`, 206 lines)

#### Test Cases
- `DistributionTestCase` — IEEE 13/34/123-bus distribution test feeders
- Synthetic test case generator enhancements (583 lines added to `testcases/synthetic.rs`)
- Integrated Resource Planning test suite (`planning/integrated/tests.rs`, 795 lines)
- Additional IEEE test system data (396 lines added to `testcases/ieee.rs`)
- Geospatial integration tests (`tests/geospatial_test.rs`)

#### Units
- `EnergyUnit` — structured energy unit conversions (Wh, kWh, MWh, GWh, BTU, GJ)
- `ThermalUnit` — thermal conductance and resistance unit conversions

### Changed
- `timeseries_sim` and `grid_ops` modules refactored from single oversized files into structured
  submodules, improving navigability and reducing per-file line count
- `zone_protection` refactored to a module with dedicated `coordination.rs` (966 lines)
- `planning/integrated` refactored to a module with a comprehensive test suite
- `carbon_budget` market module reorganised: new types in `carbon_budget.rs`, legacy API
  preserved via `carbon_budget_legacy.rs` re-export for backward compatibility

## [0.1.1] - 2026-05-03

### Fixed
- Disambiguate `solve_dense` method call in `SimdAvx2Backend` — resolves E0034 "multiple applicable
  items in scope" compilation error by using `LinearSolver::solve_dense` UFCS syntax instead of
  bare `self.inner.solve_dense` when both `LinearAlgebraBackend` and `LinearSolver` traits are in
  scope (`src/powerflow/linalg.rs`)

### Added
- PSS designer module with automated parameter tuning (`src/stability/pss_design/`)
- Linear algebra backend trait abstraction for power flow solvers (`src/powerflow/linalg.rs`)
- Asset digitization types for power grid assets (`src/digitaltwin/asset_digitization/`)
- Expanded unit tests: EV-grid integration, distribution planning, power quality compliance

### Changed
- Battery ECM and digital twin modules refactored for improved structure
- Code structure improved for readability and maintainability across multiple modules
- 5,036 tests now passing (up from 3,830 at v0.1.0)

## [0.1.0] - 2026-03-09

### Added

#### Power Flow
- Newton-Raphson power flow solver with warm-start support
- Fast decoupled load flow (FDLF)
- DC power flow
- Holomorphic embedding load flow method (HELM) with Padé acceleration
- Continuation power flow with nose-point detection
- Probabilistic power flow (Monte Carlo, Point Estimate Method)
- AC branch flows (π-model) and DC branch flows
- SIMD-accelerated NR kernels (AVX2, behind `simd` feature)
- Three-phase unbalanced Newton-Raphson power flow
- AC/DC Weighted Least-Squares state estimation
- EKF dynamic state estimator

#### Network
- Y-bus admittance matrix construction
- Network topology analysis (petgraph-based)
- N-1/N-k contingency analysis
- HVDC links (LCC/VSC) and multi-terminal DC grids
- FACTS models (STATCOM, SVC, TCSC, UPFC)
- Distribution network reconfiguration
- Transformer models (two-winding, three-winding, OLTC, IEC 60076-7 thermal)
- Voltage regulation (regulators, capacitor banks, SVC, coordinated VVC)
- Grid resilience planning (N-k analysis, hardening optimizer)

#### Stability
- Transient stability (RK45 adaptive, event queue, CCT finder)
- Small signal stability analysis (Householder QR + Francis double-shift)
- Voltage stability (L-index, modal analysis, FVSI, VSA)
- Multi-machine transient stability
- AGC + frequency regulation
- Grid-forming inverter dynamics (VSM swing equation)
- Load modeling (ZIP, Exponential, Motor, Composite WECC CLOD, Restorative)

#### Battery
- Equivalent Circuit Model (ECM) with 2-RC network
- P2D Doyle-Fuller-Newman (DFN) electrochemical model
- Thermal model (1D finite-difference + lumped)
- Battery aging and SoH estimation (EKF)
- Battery Management System (BMS) with fault detection
- Pack modeling (cell/module/pack with thermal derating)

#### Renewable Energy
- Solar PV (irradiance, MPPT, inverter, shading)
- Wind (turbine, wake models, offshore 15 MW, farm layout optimization)
- Renewable grid codes (LVRT/HVRT, FCR/FRR, PQ diagram, ramp limits)
- Forecasting: ARIMA/SARIMA, ensemble, weather-coupled NWP, conformal prediction intervals
- Integration analysis (hosting capacity, SCR/WSCR, inertia/ROCOF)

#### Optimization
- DC-OPF (lambda-iteration + LP via OxiZ)
- AC-OPF
- SCOPF (N-1 security-constrained)
- ORPD (optimal reactive power dispatch)
- Multi-period OPF with ramp constraints
- MILP unit commitment (Branch-and-Bound)
- MPC energy management system
- Microgrid EMS with advanced multi-objective optimizer
- Demand response (flexibility portfolio, market clearing, program management)
- Energy storage: price arbitrage, multi-market, degradation-aware scheduling, BESS fleet
- EV charging (smart charging, V2G, fleet coordination, grid integration)
- Hydrogen/P2G (electrolyzer, storage, fuel cell, seasonal storage)
- Market clearing (DAM, RTM, ancillary services, LMP, DSO flexibility, carbon budget)
- Distribution expansion planning, DER integration planning

#### Protection
- Fault analysis (3-phase, SLG, LL, DLG — IEC 60909)
- Protection relay coordination (IDMT curves, CTI grading)
- Differential protection 87T/87B
- Distance protection with zone coordination
- Motor protection (NEMA/IEC thermal overload)
- Fault current limiters (SFCL, resistive, bridge-type)
- IBR protection (distance reach correction, ROCOF, LOM)
- Relay testing (IEC IDMT, pickup, CTI acceptance reports)

#### Harmonics
- THD/spectrum analysis (OxiFFT-based)
- Harmonic standards compliance (IEEE 519, EN 50160, IEC 61000-3-2)
- Passive/active filter design
- Flicker analysis (Pst/Plt)

#### Power Quality
- Event classification (IEEE 1159) with ML classifier (k-NN + CART decision tree)
- PQ indices (THD, K-factor, crest factor)
- Sag/swell detection (ITIC/SEMI F47)
- Standards compliance checker (EN 50160, IEEE 519, NERC TPL)

#### Digital Twin
- Grid digital twin (WLS SE + NR PF, SCADA/PMU telemetry)
- Asset digitization and lifecycle management
- Alert engine and grid replay

#### Security
- Intrusion detection system (IDS)
- Vulnerability assessment
- Cyber-physical attack simulation (FDI, DoS, Monte Carlo)
- Threat intelligence (MITRE ATT&CK for ICS, anomaly detection, NERC CIP)

#### Analytics
- Carbon intensity forecast and LME
- Grid operations KPIs
- Energy equity analysis

#### Monitoring
- Frequency monitoring (ROCOF relay, UFLS, nadir estimator, inertia estimation)

#### IO
- CSV import/export
- MATPOWER format export
- PMU synchrophasor processing
- Time-series data management

#### Test Cases
- IEEE 14/30/57/118/300-bus standard test systems
- RTS-96, PEGASE-89
- Synthetic topologies (Ring/Radial/Meshed/Geographic/SmallWorld/ScaleFree)
- Benchmark suite with reference solutions

[0.1.2]: https://github.com/cool-japan/oxigrid/releases/tag/v0.1.2
[0.1.1]: https://github.com/cool-japan/oxigrid/releases/tag/v0.1.1
[0.1.0]: https://github.com/cool-japan/oxigrid/releases/tag/v0.1.0
