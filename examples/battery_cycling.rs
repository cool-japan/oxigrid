use oxigrid::battery::ecm::{ParameterSet, TwoRcModel};
use oxigrid::battery::soc::{CoulombCounter, EkfSocEstimator};
use oxigrid::battery::thermal::LumpedThermalModel;
use oxigrid::battery::BatteryModel;
use oxigrid::battery::OcvSocCurve;
use oxigrid::units::{Current, Temperature};

fn main() {
    println!("Battery Cycling Example — Kokam 75 Ah LFP Cell");
    println!("=================================================");

    let p = ParameterSet::kokam_75ah_lfp();
    let curve = OcvSocCurve::lfp_default();

    let mut model =
        TwoRcModel::new(curve.clone(), p.r0, p.r1, p.c1, p.r2, p.c2, p.capacity_ah).with_soc(1.0);

    let mut cc = CoulombCounter::new(1.0, p.capacity_ah);
    let mut ekf = EkfSocEstimator::new(curve.clone(), p.r0, p.capacity_ah, 1.0);
    let mut thermal = LumpedThermalModel::pouch_75ah();

    let i_discharge = Current(p.capacity_ah); // 1C discharge
    let i_charge = Current(-p.capacity_ah * 0.5); // C/2 charge
    let dt = 10.0; // 10-second timesteps
    let t_ref = Temperature(298.15);

    let mut cycle = 0usize;
    let max_cycles = 3;

    println!(
        "\n{:>5}  {:>8}  {:>8}  {:>8}  {:>8}  {:>8}  {:>8}",
        "Step", "Phase", "SoC_ECM", "SoC_CC", "SoC_EKF", "V (V)", "T (°C)"
    );
    println!("{}", "-".repeat(65));

    let mut step = 0usize;
    while cycle < max_cycles {
        // Discharge phase
        println!("\n--- Cycle {} Discharge ---", cycle + 1);
        model.soc = 1.0;
        cc.soc = 1.0;

        loop {
            let state = model.step(i_discharge, dt, t_ref);
            cc.step(i_discharge, dt);
            ekf.update(i_discharge, state.voltage, dt, t_ref);
            thermal.step(i_discharge.0, p.r0, dt);

            if step.is_multiple_of(36) {
                println!(
                    "{:>5}  {:>8}  {:>8.4}  {:>8.4}  {:>8.4}  {:>8.4}  {:>8.2}",
                    step,
                    "DIS",
                    state.soc.0,
                    cc.soc,
                    ekf.x,
                    state.voltage.0,
                    thermal.temperature - 273.15,
                );
            }
            step += 1;
            if state.soc.0 < 0.05 {
                break;
            }
        }

        // Rest (60 s)
        for _ in 0..6 {
            model.step(Current(0.0), dt, t_ref);
            step += 1;
        }

        // Charge phase (CC to 100% SoC)
        println!("\n--- Cycle {} Charge ---", cycle + 1);
        loop {
            let state = model.step(i_charge, dt, t_ref);
            cc.step(i_charge, dt);
            ekf.update(i_charge, state.voltage, dt, t_ref);
            thermal.step(i_charge.0.abs(), p.r0, dt);

            if step.is_multiple_of(36) {
                println!(
                    "{:>5}  {:>8}  {:>8.4}  {:>8.4}  {:>8.4}  {:>8.4}  {:>8.2}",
                    step,
                    "CHG",
                    state.soc.0,
                    cc.soc,
                    ekf.x,
                    state.voltage.0,
                    thermal.temperature - 273.15,
                );
            }
            step += 1;
            if state.soc.0 > 0.99 {
                break;
            }
        }

        // Rest (60 s)
        for _ in 0..6 {
            model.step(Current(0.0), dt, t_ref);
            step += 1;
        }

        cycle += 1;
    }

    println!("\n--- Final State after {} cycles ---", max_cycles);
    let state = model.state();
    println!("  SoC (ECM):  {:.4}", state.soc.0);
    println!("  Voltage:    {:.4} V", state.voltage.0);
    println!("  SoC (CC):   {:.4}", cc.soc);
    println!("  SoC (EKF):  {:.4}", ekf.x);
    println!("  Cell temp:  {:.2} °C", thermal.temperature - 273.15);

    // Energy throughput report
    println!(
        "\nSummary: {} cycles completed in {} time steps ({:.1} h simulated)",
        max_cycles,
        step,
        step as f64 * dt / 3600.0
    );
}
