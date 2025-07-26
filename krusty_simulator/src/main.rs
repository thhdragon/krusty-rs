use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

// Add csv crate
use csv::Writer;
// Add serde and serde_json for JSONL output
use serde::Serialize;
use serde_json;
use krusty_shared::{HeaterState, StepGenerator, StepCommand, ThermistorState, FanState, SwitchState, Position, HardwareState};
use config as config_rs;
use serde::Deserialize;
use crossbeam_channel::{unbounded, Sender};
use krusty_shared::gcode_utils::{parse_gcode_line, parse_loop_conditional, parse_hardware_command};

#[derive(Debug, Serialize)]
struct StepRecord {
    step: usize,
    x: f64,
    y: f64,
    z: f64,
    e: f64,
    vx: f64,
    vy: f64,
    vz: f64,
    ve: f64,
    ax: f64,
    ay: f64,
    az: f64,
    ae: f64,
    heater_on: bool,
    fan_on: bool,
    switch_on: bool,
    gcode: String,
}

// STUB: Dummy hardware abstractions for future integration
#[derive(Debug, Clone)]
struct Heater {} // STUB: Not yet connected to simulation
#[derive(Debug, Clone)]
struct Fan {}    // STUB: Not yet connected to simulation
#[derive(Debug, Clone)]
struct Switch {} // STUB: Not yet connected to simulation

#[derive(Debug, Deserialize)]
struct SimConfig {
    simulation: SimulationConfig,
    motion: MotionConfig,
    hardware: HardwareConfig,
}

#[derive(Debug, Deserialize)]
struct SimulationConfig {
    gcode: String,
    output_dir: String,
}

#[derive(Debug, Deserialize)]
struct MotionConfig {
    axes: Vec<String>,
    profile: String,
    max_speed: f64,
    max_accel: f64,
    max_jerk: f64,
    steps_per_mm: [f64; 4],
    direction_invert: [bool; 4],
}

#[derive(Debug, Deserialize)]
struct HardwareConfig {
    board: String,
}

fn write_step_output(
    wtr: &mut Writer<File>,
    jsonl_file: &mut Option<File>,
    step: usize,
    pos: &Position,
    vx: f64,
    vy: f64,
    vz: f64,
    ve: f64,
    ax: f64,
    ay: f64,
    az: f64,
    ae: f64,
    hw_state: &HardwareState,
    gcode: &str,
) {
    println!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\",{:.2},{:.2}",
        step,
        pos.x,
        pos.y,
        pos.z,
        pos.e,
        vx,
        vy,
        vz,
        ve,
        ax,
        ay,
        az,
        ae,
        hw_state.heater_on,
        hw_state.fan_on,
        hw_state.switch_on,
        gcode,
        hw_state.heater_state.current_temp,
        hw_state.thermistor_state.measured_temp
    );
    wtr.write_record(&[
        step.to_string(),
        pos.x.to_string(),
        pos.y.to_string(),
        pos.z.to_string(),
        pos.e.to_string(),
        vx.to_string(),
        vy.to_string(),
        vz.to_string(),
        ve.to_string(),
        ax.to_string(),
        ay.to_string(),
        az.to_string(),
        ae.to_string(),
        hw_state.heater_on.to_string(),
        hw_state.fan_on.to_string(),
        hw_state.switch_on.to_string(),
        gcode.to_string(),
    ]).expect("Failed to write record");
    if let Some(jsonl) = jsonl_file {
        let record = StepRecord {
            step,
            x: pos.x,
            y: pos.y,
            z: pos.z,
            e: pos.e,
            vx,
            vy,
            vz,
            ve,
            ax,
            ay,
            az,
            ae,
            heater_on: hw_state.heater_on,
            fan_on: hw_state.fan_on,
            switch_on: hw_state.switch_on,
            gcode: gcode.to_string(),
        };
        let json = serde_json::to_string(&record).expect("Failed to serialize to JSON");
        writeln!(jsonl, "{}", json).expect("Failed to write to JSONL file");
    }
    println!(
        "HW: heater={}, fan={}, switch={}",
        hw_state.heater_on, hw_state.fan_on, hw_state.switch_on
    );
}

#[derive(Debug, Clone)]
enum EventType {
    HardwareChanged(HardwareState),
    GCodeParsed(String),
    StepGenerated(Vec<StepCommand>),
    SimulationStep(usize),
}

fn main() {
    // --- Config loading ---
    let settings = config_rs::Config::builder()
        .add_source(config_rs::File::with_name("krusty_simulator/test_sim.toml"))
        .build()
        .expect("Failed to load config");
    let sim_config: SimConfig = settings.try_deserialize().expect("Invalid config format");
    println!("Loaded config: {:#?}", sim_config);

    // Use config values instead of hardcoded paths
    let input_path = &sim_config.simulation.gcode;
    let output_path = &format!("{}results.csv", sim_config.simulation.output_dir);
    let jsonl_path = Some(format!("{}results.jsonl", sim_config.simulation.output_dir));
    let file = File::open(input_path).expect("Failed to open GCode file");
    let reader = BufReader::new(file);
    let mut pos = Position::default();
    let mut step = 1;
    let mut prev_pos = Position::default();
    let mut prev_vx = 0.0;
    let mut prev_vy = 0.0;
    let mut prev_vz = 0.0;
    let mut prev_ve = 0.0;
    let dt = 1.0; // TODO: Make configurable if needed
    println!("step,x,y,z,e,vx,vy,vz,ve,ax,ay,az,ae,heater_on,fan_on,switch_on,gcode");
    let mut wtr = Writer::from_path(output_path).expect("Failed to create CSV writer");
    wtr.write_record(["step", "x", "y", "z", "e", "vx", "vy", "vz", "ve", "ax", "ay", "az", "ae", "heater_on", "fan_on", "switch_on", "gcode"]).expect("Failed to write header");
    let mut jsonl_file = if let Some(path) = jsonl_path {
        Some(File::create(path).expect("Failed to create JSONL file"))
    } else {
        None
    };
    // Use motion config for stepgen (axes, etc.)
    let mut hw_state = HardwareState {
        heater_on: false,
        fan_on: false,
        switch_on: false,
        heater_state: HeaterState {
            power: 0.0,
            target_temp: 200.0,
            current_temp: 25.0,
            is_on: false,
            runaway_detected: false,
            runaway_check_timer: 0.0,
            runaway_enabled: false,
        },
        thermistor_state: ThermistorState {
            measured_temp: 25.0,
            noise: 0.5,
            last_update: 0.0,
        },
        fan_state: FanState { power: 0.0, is_on: false, rpm: 0.0 },
        switch_state: SwitchState { is_on: false, debounce_counter: 0 },
    };
    let mut stepgen = StepGenerator::new(
        sim_config.motion.steps_per_mm,
        sim_config.motion.direction_invert,
    );
    // Set up event bus
    let (event_tx, event_rx) = unbounded::<EventType>();
    // Spawn a thread to consume and log events
    std::thread::spawn({
        let event_rx = event_rx.clone();
        move || {
            while let Ok(event) = event_rx.recv() {
                match event {
                    EventType::HardwareChanged(hw) => println!("[EVENT] HardwareChanged: {:?}", hw),
                    EventType::GCodeParsed(line) => println!("[EVENT] GCodeParsed: {}", line),
                    EventType::StepGenerated(cmds) => println!("[EVENT] StepGenerated: {:?}", cmds),
                    EventType::SimulationStep(step) => println!("[EVENT] SimulationStep: {}", step),
                }
            }
        }
    });
    for line in reader.lines() {
        let line = line.unwrap();
        if line.trim().is_empty() || line.starts_with(';') {
            continue;
        }
        parse_hardware_command(&line, &mut hw_state);
        // Emit hardware change event
        event_tx.send(EventType::HardwareChanged(hw_state.clone())).ok();
        // Simulate heater/thermistor physics
        let dt_f32 = dt as f32;
        let ambient = 22.0;
        let _event = hw_state.heater_state.update(dt_f32, ambient);
        hw_state.thermistor_state.update(hw_state.heater_state.current_temp, dt_f32);
        hw_state.fan_state.update(dt_f32);
        hw_state.switch_state.update(dt_f32);
        // Emit GCode parsed event
        event_tx.send(EventType::GCodeParsed(line.clone())).ok();
        let (loop_until_hit, conditional) = parse_loop_conditional(&line, &pos);
        let new_pos = parse_gcode_line(&line, &pos);
        if loop_until_hit {
            let mut loop_pos = new_pos.clone();
            while loop_pos.x < 10.0 {
                let vx = (loop_pos.x - pos.x) / dt;
                let vy = (loop_pos.y - pos.y) / dt;
                let vz = (loop_pos.z - pos.z) / dt;
                let ve = (loop_pos.e - pos.e) / dt;
                let ax = (vx - prev_vx) / dt;
                let ay = (vy - prev_vy) / dt;
                let az = (vz - prev_vz) / dt;
                let ae = (ve - prev_ve) / dt;
                // Generate stepper commands using shared logic
                let step_cmds = stepgen.generate_steps(&[loop_pos.x, loop_pos.y, loop_pos.z, loop_pos.e]);
                // Emit step generated event
                event_tx.send(EventType::StepGenerated(step_cmds.clone())).ok();
                // Emit simulation step event
                event_tx.send(EventType::SimulationStep(step)).ok();
                println!("Stepper commands: {:?}", step_cmds);
                write_step_output(
                    &mut wtr,
                    &mut jsonl_file,
                    step,
                    &loop_pos,
                    vx,
                    vy,
                    vz,
                    ve,
                    ax,
                    ay,
                    az,
                    ae,
                    &hw_state,
                    &line,
                );
                prev_vx = vx;
                prev_vy = vy;
                prev_vz = vz;
                prev_ve = ve;
                pos = loop_pos.clone(); // Fix position tracking
                step += 1;
                loop_pos.x += 1.0;
            }
            continue;
        }
        if conditional && pos.y <= 20.0 {
            continue;
        }
        let vx = (new_pos.x - pos.x) / dt;
        let vy = (new_pos.y - pos.y) / dt;
        let vz = (new_pos.z - pos.z) / dt;
        let ve = (new_pos.e - pos.e) / dt;
        let ax = (vx - prev_vx) / dt;
        let ay = (vy - prev_vy) / dt;
        let az = (vz - prev_vz) / dt;
        let ae = (ve - prev_ve) / dt;
        // Generate stepper commands using shared logic
        let step_cmds = stepgen.generate_steps(&[new_pos.x, new_pos.y, new_pos.z, new_pos.e]);
        // Emit step generated event
        event_tx.send(EventType::StepGenerated(step_cmds.clone())).ok();
        // Emit simulation step event
        event_tx.send(EventType::SimulationStep(step)).ok();
        println!("Stepper commands: {:?}", step_cmds);
        write_step_output(
            &mut wtr,
            &mut jsonl_file,
            step,
            &new_pos,
            vx,
            vy,
            vz,
            ve,
            ax,
            ay,
            az,
            ae,
            &hw_state,
            &line,
        );
        prev_vx = vx;
        prev_vy = vy;
        prev_vz = vz;
        prev_ve = ve;
        prev_pos = new_pos.clone();
        pos = new_pos;
        step += 1;
    }
    wtr.flush().expect("Failed to flush CSV writer");
}
