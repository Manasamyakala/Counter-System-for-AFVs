use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    cursor::{Hide, Show},
    event::{poll, read, Event, KeyCode},
};
use crate::simulator::{SimulationState, DroneStatus, JammingType, DroneType, ThreatLevel};

pub async fn run_tui(state: Arc<RwLock<SimulationState>>) {
    // 1. Set up terminal raw mode and alternate screen
    enable_raw_mode().unwrap();
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide).unwrap();

    let mut selected_idx = 0;
    let mut drone_ids: Vec<String> = Vec::new();

    let mut interval = tokio::time::interval(Duration::from_millis(150));

    loop {
        // Handle input events first (non-blocking poll)
        if poll(Duration::from_millis(10)).unwrap() {
            if let Event::Key(key) = read().unwrap() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Tab => {
                        if !drone_ids.is_empty() {
                            selected_idx = (selected_idx + 1) % drone_ids.len();
                        }
                    }
                    KeyCode::Char('1') => {
                        // RF Jamming on selected
                        if !drone_ids.is_empty() && selected_idx < drone_ids.len() {
                            let target_id = drone_ids[selected_idx].clone();
                            let mut lock = state.write().await;
                            lock.trigger_soft_kill(&target_id, JammingType::RfJamming);
                        }
                    }
                    KeyCode::Char('2') => {
                        // GPS Spoofing on selected
                        if !drone_ids.is_empty() && selected_idx < drone_ids.len() {
                            let target_id = drone_ids[selected_idx].clone();
                            let mut lock = state.write().await;
                            lock.trigger_soft_kill(&target_id, JammingType::GpsSpoofing);
                        }
                    }
                    KeyCode::Char('3') => {
                        // EMP on selected
                        if !drone_ids.is_empty() && selected_idx < drone_ids.len() {
                            let target_id = drone_ids[selected_idx].clone();
                            let mut lock = state.write().await;
                            lock.trigger_soft_kill(&target_id, JammingType::Emp);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Fetch simulation snapshot
        let snap = {
            let lock = state.read().await;
            lock.clone()
        };

        // Update list of active/jammable drone IDs
        drone_ids = snap.drones.values()
            .filter(|d| match d.status {
                DroneStatus::Active | DroneStatus::Jamming { .. } => true,
                _ => false
            })
            .map(|d| d.id.clone())
            .collect();
        
        if !drone_ids.is_empty() {
            selected_idx = selected_idx.min(drone_ids.len() - 1);
        } else {
            selected_idx = 0;
        }

        let selected_id = if !drone_ids.is_empty() {
            Some(drone_ids[selected_idx].clone())
        } else {
            None
        };

        // Clear terminal screen and position cursor at 1,1
        print!("\x1B[2J\x1B[1;1H");

        // RENDER TUI
        println!("\x1B[1m\x1B[32m=======================================================================================\x1B[0m");
        println!("\x1B[1m\x1B[32m           AFV SHIELD - ANTI-DRONE MILITARY COMMAND HUD (TERMINAL EDITION)\x1B[0m");
        println!("\x1B[1m\x1B[32m=======================================================================================\x1B[0m");

        // Print AFV Status Panel
        println!(
            " \x1B[1m[AFV STATUS]\x1B[0m Vehicle: \x1B[37m{}\x1B[0m | Coords: \x1B[37mX:{:.1} Y:{:.1}\x1B[0m | Speed: \x1B[37m{:.1} km/h\x1B[0m | Heading: \x1B[37m{:.0}°\x1B[0m",
            snap.afv.name, snap.afv.x, snap.afv.y, snap.afv.speed * 3.6, snap.afv.heading
        );
        println!("\x1B[32m---------------------------------------------------------------------------------------\x1B[0m");

        // RENDER CANVAS BUFFERS
        let mut radar = vec![vec![" . "; 15]; 15];
        let mut elevation = vec![vec![" . "; 20]; 15];

        // Draw axes on radar
        for r in 0..15 {
            radar[r][7] = " | ";
            radar[7][r] = "---";
        }
        radar[7][7] = "\x1B[32m H \x1B[0m"; // AFV Home Center

        // Draw axes on elevation
        for r in 0..15 {
            elevation[r][0] = " | ";
        }
        for c in 0..20 {
            elevation[14][c] = "---";
        }
        elevation[14][0] = "\x1B[32m H \x1B[0m"; // AFV

        // Map drones to buffers
        for drone in snap.drones.values() {
            // Radar Mapping (X, Y in [-3000, 3000] -> [0, 14])
            let rx = (7.0 + (drone.x / 3000.0 * 7.0)).round() as i32;
            let ry = (7.0 - (drone.y / 3000.0 * 7.0)).round() as i32;
            if rx >= 0 && rx < 15 && ry >= 0 && ry < 15 {
                let symbol = match drone.status {
                    DroneStatus::Active => match drone.drone_type {
                        DroneType::Surveillance => "\x1B[34m S \x1B[0m", // Blue
                        DroneType::Attack => "\x1B[31m A \x1B[0m",       // Red
                        DroneType::Swarm => "\x1B[33m W \x1B[0m",        // Yellow
                    },
                    DroneStatus::Jamming { .. } => "\x1B[36m * \x1B[0m",  // Cyan
                    _ => "\x1B[90m X \x1B[0m",                           // Grey
                };
                radar[ry as usize][rx as usize] = symbol;
            }

            // Elevation Mapping (Dist2D in [0, 3000] -> [0, 19], Z in [0, 500] -> [0, 14])
            let ex = ((drone.distance_2d / 3000.0 * 19.0)).round() as i32;
            let ey = (14.0 - (drone.z / 500.0 * 14.0)).round() as i32;
            if ex >= 0 && ex < 20 && ey >= 0 && ey < 15 {
                let symbol = match drone.status {
                    DroneStatus::Active => match drone.drone_type {
                        DroneType::Surveillance => "\x1B[34m S \x1B[0m",
                        DroneType::Attack => "\x1B[31m A \x1B[0m",
                        DroneType::Swarm => "\x1B[33m W \x1B[0m",
                    },
                    DroneStatus::Jamming { .. } => "\x1B[36m * \x1B[0m",
                    _ => "\x1B[90m X \x1B[0m",
                };
                elevation[ey as usize][ex as usize] = symbol;
            }
        }

        // Print views side by side
        println!(" \x1B[1m[TOP VIEW - RADAR HORIZONTAL]\x1B[0m          \x1B[1m[SIDE VIEW - ELEVATION PROFILE]\x1B[0m");
        println!("   (Range Rmax = 3000m)                       (Ground Range 3km, Alt 500m)");
        for r in 0..15 {
            // Print Radar Row
            print!("   ");
            for c in 0..15 {
                print!("{}", radar[r][c]);
            }
            // Print spacing
            print!("         ");
            // Print Elevation Row
            for c in 0..20 {
                print!("{}", elevation[r][c]);
            }
            println!();
        }
        println!("\x1B[32m---------------------------------------------------------------------------------------\x1B[0m");

        // Print Target Matrix Table
        println!(" \x1B[1m[TARGET TELEMETRY MATRIX]\x1B[0m");
        println!(" ID          Type         Threat    3D Range  Altitude  Azimuth   Elevation Status");
        println!(" -------------------------------------------------------------------------------------");
        
        for drone in snap.drones.values() {
            let is_sel = Some(drone.id.clone()) == selected_id;
            let select_marker = if is_sel { "\x1B[1m\x1B[37m>\x1B[0m" } else { " " };
            
            let color_code = match drone.status {
                DroneStatus::Neutralized | DroneStatus::Crashed => "\x1B[90m", // Grey
                DroneStatus::Jamming { .. } => "\x1B[36m",                     // Cyan
                DroneStatus::Active => match drone.threat_level {
                    ThreatLevel::Critical => "\x1B[1m\x1B[31m",                // Bold Red
                    ThreatLevel::High => "\x1B[33m",                           // Yellow
                    ThreatLevel::Medium => "\x1B[93m",                          // Light Yellow
                    ThreatLevel::Low => "\x1B[34m",                            // Blue
                }
            };

            let type_str = match drone.drone_type {
                DroneType::Surveillance => "SURVEILLANCE",
                DroneType::Attack => "ATTACK",
                DroneType::Swarm => "SWARM",
            };

            let threat_str = match drone.threat_level {
                ThreatLevel::Critical => "CRITICAL",
                ThreatLevel::High => "HIGH",
                ThreatLevel::Medium => "MEDIUM",
                ThreatLevel::Low => "LOW",
            };

            let status_str = match &drone.status {
                DroneStatus::Active => "ACTIVE".to_string(),
                DroneStatus::Jamming { jam_type, progress } => {
                    let j_name = match jam_type {
                        JammingType::RfJamming => "RF-JAM",
                        JammingType::GpsSpoofing => "GPS-SPOOF",
                        JammingType::Emp => "EMP-KILL",
                    };
                    format!("{}({:.0}%)", j_name, progress * 100.0)
                }
                DroneStatus::Neutralized | DroneStatus::Crashed => "DOWN".to_string(),
            };

            println!(
                "{} {}{:11} {:12} {:9} {:7.1}m  {:7.1}m  {:7.1}°   {:7.1}°   {:11}\x1B[0m",
                select_marker,
                color_code,
                drone.id,
                type_str,
                threat_str,
                drone.distance_3d,
                drone.z,
                drone.azimuth,
                drone.elevation,
                status_str
            );
        }
        println!("\x1B[32m---------------------------------------------------------------------------------------\x1B[0m");

        // Print Logs
        println!(" \x1B[1m[TACTICAL ALERTS & LOGS]\x1B[0m");
        let start_log_idx = snap.logs.len().saturating_sub(4);
        for l in start_log_idx..snap.logs.len() {
            println!("   {}", snap.logs[l]);
        }
        println!("\x1B[32m---------------------------------------------------------------------------------------\x1B[0m");

        // Print Controls Prompt
        println!(
            " \x1B[1m[CONTROLS]\x1B[0m \x1B[1mTab\x1B[0m: Cycle Target | \x1B[1m1\x1B[0m: RF Jam | \x1B[1m2\x1B[0m: GPS Spoof | \x1B[1m3\x1B[0m: EMP Pulse | \x1B[1mQ/Esc\x1B[0m: Exit"
        );
        print!(" SELECTED: ");
        if let Some(ref id) = selected_id {
            print!("\x1B[1m\x1B[33m{}\x1B[0m", id);
        } else {
            print!("\x1B[90mNONE (No Active Drones)\x1B[0m");
        }
        println!();

        stdout.flush().unwrap();
        interval.tick().await;
    }

    // 5. Restore terminal when exiting
    disable_raw_mode().unwrap();
    execute!(std::io::stdout(), LeaveAlternateScreen, Show).unwrap();
    println!("\x1B[32m[SYSTEM] Terminal HUD deactivated. Server shutting down.\x1B[0m");
}

// Simple stdio writing helper
use std::io::Write;
