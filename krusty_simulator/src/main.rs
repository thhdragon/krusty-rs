use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

// Add csv crate
use csv::Writer;
// Add serde and serde_json for JSONL output
use serde::Serialize;
use serde_json;

#[derive(Debug, Default, Clone, Serialize)]
struct Position {
    x: f64,
    y: f64,
    z: f64,
    e: f64,
}

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

#[derive(Debug, Default, Clone, Serialize)]
struct StepperPosition {
    x: f64,
    y: f64,
    z: f64,
    e: f64,
}

impl std::ops::Add for StepperPosition {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            e: self.e + rhs.e,
        }
    }
}

impl std::ops::Sub for StepperPosition {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            e: self.e - rhs.e,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct Command {
    pos: StepperPosition,
    loop_until_hit: bool,
    safe_stop_after: bool,
    conditional: bool,
    gcode: String,
}

// Dummy hardware abstractions
#[derive(Debug, Clone)]
struct Heater {}
#[derive(Debug, Clone)]
struct Fan {}
#[derive(Debug, Clone)]
struct Switch {}

#[derive(Debug, Default, Clone)]
struct HardwareState {
    heater_on: bool,
    fan_on: bool,
    switch_on: bool,
}

fn parse_gcode_line(line: &str, last_pos: &Position) -> Position {
    let mut pos = last_pos.clone();
    let tokens: Vec<&str> = line.split_whitespace().collect();
    for token in tokens {
        if token.starts_with("X") {
            pos.x = token[1..].parse().unwrap_or(pos.x);
        } else if token.starts_with("Y") {
            pos.y = token[1..].parse().unwrap_or(pos.y);
        } else if token.starts_with("Z") {
            pos.z = token[1..].parse().unwrap_or(pos.z);
        } else if token.starts_with("E") {
            pos.e = token[1..].parse().unwrap_or(pos.e);
        }
    }
    pos
}

fn parse_loop_conditional(line: &str, pos: &Position) -> (bool, bool) {
    // Example: ;LOOP X<10 or ;IF Y>20
    let mut loop_until_hit = false;
    let mut conditional = false;
    if line.contains(";LOOP") {
        loop_until_hit = true;
    }
    if line.contains(";IF") {
        conditional = true;
    }
    (loop_until_hit, conditional)
}

fn parse_hardware_command(line: &str, hw: &mut HardwareState) {
    if line.contains("M104") {
        hw.heater_on = true;
    }
    if line.contains("M140") {
        hw.heater_on = false;
    }
    if line.contains("M106") {
        hw.fan_on = true;
    }
    if line.contains("M107") {
        hw.fan_on = false;
    }
    if line.contains("M80") {
        hw.switch_on = true;
    }
    if line.contains("M81") {
        hw.switch_on = false;
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <input.gcode> <output.csv> [output.jsonl]", args[0]);
        return;
    }
    let input_path = &args[1];
    let output_path = &args[2];
    let jsonl_path = if args.len() > 3 { Some(&args[3]) } else { None };
    let file = File::open(input_path).expect("Failed to open GCode file");
    let reader = BufReader::new(file);
    let mut pos = Position::default();
    let mut step = 1;
    let mut prev_pos = Position::default();
    let mut prev_vx = 0.0;
    let mut prev_vy = 0.0;
    let mut prev_vz = 0.0;
    let mut prev_ve = 0.0;
    let dt = 1.0; // Assume 1 unit time per step for simplicity
    println!("step,x,y,z,e,vx,vy,vz,ve,ax,ay,az,ae,heater_on,fan_on,switch_on,gcode");
    let mut wtr = Writer::from_path(output_path).expect("Failed to create CSV writer");
    wtr.write_record(["step", "x", "y", "z", "e", "vx", "vy", "vz", "ve", "ax", "ay", "az", "ae", "heater_on", "fan_on", "switch_on", "gcode"]).expect("Failed to write header");
    let mut jsonl_file = if let Some(path) = jsonl_path {
        Some(File::create(path).expect("Failed to create JSONL file"))
    } else {
        None
    };
    let mut hw_state = HardwareState::default();
    for line in reader.lines() {
        let line = line.unwrap();
        if line.trim().is_empty() || line.starts_with(';') {
            continue;
        }
        parse_hardware_command(&line, &mut hw_state);
        let (loop_until_hit, conditional) = parse_loop_conditional(&line, &pos);
        let new_pos = parse_gcode_line(&line, &pos);
        // Simulate loop: repeat move until X < 10 (example logic)
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
                println!("{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\"", step, loop_pos.x, loop_pos.y, loop_pos.z, loop_pos.e, vx, vy, vz, ve, ax, ay, az, ae, hw_state.heater_on, hw_state.fan_on, hw_state.switch_on, line);
                wtr.write_record(&[
                    step.to_string(),
                    loop_pos.x.to_string(),
                    loop_pos.y.to_string(),
                    loop_pos.z.to_string(),
                    loop_pos.e.to_string(),
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
                    line.clone()
                ]).expect("Failed to write record");
                if let Some(ref mut jsonl) = jsonl_file {
                    let record = StepRecord {
                        step,
                        x: loop_pos.x,
                        y: loop_pos.y,
                        z: loop_pos.z,
                        e: loop_pos.e,
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
                        gcode: line.clone(),
                    };
                    let json = serde_json::to_string(&record).expect("Failed to serialize to JSON");
                    writeln!(jsonl, "{}", json).expect("Failed to write to JSONL file");
                }
                prev_vx = vx;
                prev_vy = vy;
                prev_vz = vz;
                prev_ve = ve;
                // pos = loop_pos.clone();
                step += 1;
                loop_pos.x += 1.0; // Example increment for loop simulation
            }
            pos = new_pos.clone();
            continue;
        }
        // Simulate conditional: only execute if Y > 20 (example logic)
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
        println!("{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\"", step, new_pos.x, new_pos.y, new_pos.z, new_pos.e, vx, vy, vz, ve, ax, ay, az, ae, line);
        wtr.write_record(&[
            step.to_string(),
            new_pos.x.to_string(),
            new_pos.y.to_string(),
            new_pos.z.to_string(),
            new_pos.e.to_string(),
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
            line.clone()
        ]).expect("Failed to write record");
        if let Some(ref mut jsonl) = jsonl_file {
            let record = StepRecord {
                step,
                x: new_pos.x,
                y: new_pos.y,
                z: new_pos.z,
                e: new_pos.e,
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
                gcode: line.clone(),
            };
            let json = serde_json::to_string(&record).expect("Failed to serialize to JSON");
            writeln!(jsonl, "{}", json).expect("Failed to write to JSONL file");
        }
        prev_vx = vx;
        prev_vy = vy;
        prev_vz = vz;
        prev_ve = ve;
        prev_pos = new_pos.clone();
        pos = new_pos;
        step += 1;
        // Add hardware state to output
        println!("HW: heater={}, fan={}, switch={}", hw_state.heater_on, hw_state.fan_on, hw_state.switch_on);
    }
    wtr.flush().expect("Failed to flush CSV writer");
}
