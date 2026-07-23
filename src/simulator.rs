use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DroneType {
    Surveillance,
    Attack,
    Swarm,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum JammingType {
    RfJamming,
    GpsSpoofing,
    Emp,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DroneStatus {
    Active,
    Jamming {
        jam_type: JammingType,
        progress: f64, // 0.0 to 1.0
    },
    Neutralized,
    Crashed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ThreatLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drone {
    pub id: String,
    pub drone_type: DroneType,
    pub status: DroneStatus,
    pub threat_level: ThreatLevel,
    
    // 3D Cartesian coordinates relative to AFV at (0,0,0) in meters
    pub x: f64,
    pub y: f64,
    pub z: f64,
    
    // Velocity vectors in m/s
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,

    // Spherical coordinate system telemetry (calculated)
    pub distance_3d: f64,
    pub distance_2d: f64,
    pub azimuth: f64,      // Degrees (0 to 360)
    pub elevation: f64,    // Degrees (-90 to 90)

    pub signal_strength: f64, // 0.0 to 1.0
    pub battery: f64,         // 0.0 to 100.0
    pub size_meters: f64,
    pub speed: f64,           // Total speed scalar in m/s
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfvState {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub speed: f64,
    pub heading: f64, // Degrees
    pub jammer_active: bool,
    pub jammer_range: f64, // max range in meters
    pub system_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationState {
    pub afv: AfvState,
    pub drones: HashMap<String, Drone>,
    pub logs: Vec<String>,
}

impl SimulationState {
    pub fn new() -> Self {
        let drones = HashMap::new();
        
        let mut state = Self {
            afv: AfvState {
                name: "M1A2 Abrams System-V".to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                speed: 5.2, // slowly moving forward
                heading: 45.0,
                jammer_active: false,
                jammer_range: 1500.0, // 1.5km
                system_status: "SYSTEM OK".to_string(),
            },
            drones,
            logs: vec!["[SYSTEM] Radar & Jammer defense systems online.".to_string()],
        };

        state.spawn_drone(DroneType::Surveillance, "DRN-SURV-01".to_string(), -1800.0, 1200.0, 250.0);
        state.spawn_drone(DroneType::Attack, "DRN-ATTK-02".to_string(), 2500.0, -1500.0, 180.0);
        state.spawn_drone(DroneType::Swarm, "DRN-SWRM-03A".to_string(), 1200.0, 1400.0, 80.0);
        state.spawn_drone(DroneType::Swarm, "DRN-SWRM-03B".to_string(), 1250.0, 1450.0, 85.0);
        state.spawn_drone(DroneType::Swarm, "DRN-SWRM-03C".to_string(), 1180.0, 1420.0, 75.0);

        state
    }

    pub fn spawn_drone(&mut self, drone_type: DroneType, id: String, x: f64, y: f64, z: f64) {
        let (vx, vy, vz, size_meters) = match drone_type {
            DroneType::Surveillance => {
                // Orbiting velocity direction
                let speed = 15.0; // 15 m/s
                let angle = y.atan2(x) + PI/2.0;
                (angle.cos() * speed, angle.sin() * speed, 0.0, 1.8)
            }
            DroneType::Attack => {
                // Moving straight towards AFV (0,0,0)
                let speed = 32.0; // 32 m/s
                let dir_x = -x;
                let dir_y = -y;
                let dir_z = -z;
                let len = (dir_x*dir_x + dir_y*dir_y + dir_z*dir_z).sqrt();
                (dir_x/len * speed, dir_y/len * speed, dir_z/len * speed, 1.2)
            }
            DroneType::Swarm => {
                // Moving collectively but with random offsets towards AFV
                let speed = 18.0;
                let dir_x = -x;
                let dir_y = -y;
                let len = (dir_x*dir_x + dir_y*dir_y).sqrt();
                (dir_x/len * speed, dir_y/len * speed, (rand_offset() - 0.5) * 2.0, 0.6)
            }
        };

        let mut d = Drone {
            id: id.clone(),
            drone_type,
            status: DroneStatus::Active,
            threat_level: ThreatLevel::Low,
            x,
            y,
            z,
            vx,
            vy,
            vz,
            distance_3d: 0.0,
            distance_2d: 0.0,
            azimuth: 0.0,
            elevation: 0.0,
            signal_strength: 1.0,
            battery: 100.0,
            size_meters,
            speed: (vx*vx + vy*vy + vz*vz).sqrt(),
        };
        d.update_metrics();
        d.threat_level = d.evaluate_threat();
        
        self.drones.insert(id.clone(), d);
        self.add_log(format!("[DETECT] New {} drone {} spotted at {:.0}m range.", 
            format_type(drone_type), id, distance_3d(x, y, z)));
    }

    pub fn update(&mut self, dt: f64) {
        // Increment AFV time-based properties if necessary
        // Simulating slow movement
        let afv_rad = self.afv.heading.to_radians();
        self.afv.x += afv_rad.cos() * self.afv.speed * dt;
        self.afv.y += afv_rad.sin() * self.afv.speed * dt;

        let mut to_remove = Vec::new();
        let mut to_spawn = Vec::new();
        let mut logs_to_add = Vec::new();

        // Update each drone
        for (id, drone) in self.drones.iter_mut() {
            // Subtract AFV movement to keep AFV at relative (0,0,0)
            let relative_vx = drone.vx - (self.afv.heading.to_radians().cos() * self.afv.speed);
            let relative_vy = drone.vy - (self.afv.heading.to_radians().sin() * self.afv.speed);
            let relative_vz = drone.vz;

            // Handle Jamming kinetics
            match &mut drone.status {
                DroneStatus::Active => {
                    // Normal flight
                    drone.x += relative_vx * dt;
                    drone.y += relative_vy * dt;
                    drone.z += relative_vz * dt;

                    // Battery decay
                    drone.battery = (drone.battery - 0.05 * dt).max(0.0);

                    // Add swarm noise
                    if drone.drone_type == DroneType::Swarm {
                        drone.x += (rand_offset() - 0.5) * 5.0 * dt;
                        drone.y += (rand_offset() - 0.5) * 5.0 * dt;
                        drone.z += (rand_offset() - 0.5) * 2.0 * dt;
                    }

                    // Special steering for Surveillance: keep orbiting
                    if drone.drone_type == DroneType::Surveillance {
                        let r = (drone.x*drone.x + drone.y*drone.y).sqrt();
                        let mut theta = drone.y.atan2(drone.x);
                        theta += 0.05 * dt; // angular speed
                        drone.x = theta.cos() * r;
                        drone.y = theta.sin() * r;
                    }

                    // If attack drone gets within 50m, it detonates!
                    if drone.drone_type == DroneType::Attack && drone.distance_3d < 50.0 {
                        to_remove.push(id.clone());
                        to_spawn.push((DroneType::Attack, format!("DRN-ATTK-{:02}", rand_id())));
                    }
                }
                DroneStatus::Jamming { jam_type, progress } => {
                    // Drift out of control, lose altitude
                    *progress = (*progress + 0.25 * dt).min(1.0);
                    drone.signal_strength = (1.0 - *progress).max(0.0);
                    
                    // Drone falls and drifts erratically
                    let fall_rate = match jam_type {
                        JammingType::RfJamming => 15.0, // fast descent
                        JammingType::GpsSpoofing => 8.0,  // controlled drift downwards
                        JammingType::Emp => 45.0,         // immediate freefall
                    };

                    drone.z = (drone.z - fall_rate * dt).max(0.0);
                    drone.x += (rand_offset() - 0.5) * 20.0 * dt;
                    drone.y += (rand_offset() - 0.5) * 20.0 * dt;

                    if drone.z <= 0.0 {
                        drone.z = 0.0;
                        drone.status = DroneStatus::Neutralized;
                        logs_to_add.push(format!("[KILL] Drone {} successfully neutralized (Crashed to Ground).", id));
                    }
                }
                DroneStatus::Neutralized | DroneStatus::Crashed => {
                    // Drone is static on the ground, does not move
                    // We keep it for visual history but can clean up after a while
                }
            }

            drone.update_metrics();
            drone.threat_level = drone.evaluate_threat();
        }

        // Apply collected logs
        for log in logs_to_add {
            self.add_log(log);
        }

        // Clean up or respawn drones
        for id in to_remove {
            self.drones.remove(&id);
            self.add_log(format!("[IMPACT] Threat {} reached proximity. Impact detected!", id));
        }

        for (drone_type, id) in to_spawn {
            // Spawn far away
            let angle = rand_offset() * 2.0 * PI;
            let dist = 2800.0;
            self.spawn_drone(drone_type, id, angle.cos() * dist, angle.sin() * dist, 150.0 + rand_offset() * 100.0);
        }

        // If no active attack/swarm drones, spawn a new one occasionally
        let active_count = self.drones.values().filter(|d| d.status == DroneStatus::Active).count();
        if active_count < 3 {
            let r = rand_offset();
            if r < 0.33 {
                let id = format!("DRN-SURV-{:02}", rand_id());
                self.spawn_drone(DroneType::Surveillance, id, -2000.0, -2000.0, 220.0);
            } else if r < 0.66 {
                let id = format!("DRN-ATTK-{:02}", rand_id());
                self.spawn_drone(DroneType::Attack, id, 2500.0, 2000.0, 200.0);
            } else {
                let id_a = format!("DRN-SWRM-{:02}A", rand_id());
                let id_b = format!("DRN-SWRM-{:02}B", rand_id());
                self.spawn_drone(DroneType::Swarm, id_a, -1500.0, 1500.0, 90.0);
                self.spawn_drone(DroneType::Swarm, id_b, -1530.0, 1530.0, 95.0);
            }
        }
    }

    pub fn trigger_soft_kill(&mut self, drone_id: &str, jam_type: JammingType) -> bool {
        let mut activated = false;
        let mut dist = 0.0;

        if let Some(drone) = self.drones.get_mut(drone_id) {
            if drone.status == DroneStatus::Active {
                drone.status = DroneStatus::Jamming {
                    jam_type,
                    progress: 0.0,
                };
                dist = drone.distance_3d;
                activated = true;
            }
        }

        if activated {
            self.afv.jammer_active = true;
            let jam_name = match jam_type {
                JammingType::RfJamming => "RF Signal Jamming",
                JammingType::GpsSpoofing => "GPS Spoofing Protocol",
                JammingType::Emp => "High-Power EMP Pulse",
            };
            self.add_log(format!("[ACTION] {} activated against drone {} (Range: {:.1}m).", 
                jam_name, drone_id, dist));
            true
        } else {
            false
        }
    }

    pub fn add_log(&mut self, log: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.logs.push(format!("[{}] {}", timestamp, log));
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }
}

impl Drone {
    pub fn update_metrics(&mut self) {
        // Calculate distances
        self.distance_2d = (self.x * self.x + self.y * self.y).sqrt();
        self.distance_3d = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();

        // Azimuth in degrees: 0 to 360 (measured from positive X-axis counter-clockwise)
        let mut az = self.y.atan2(self.x).to_degrees();
        if az < 0.0 {
            az += 360.0;
        }
        self.azimuth = az;

        // Elevation in degrees: -90 to 90
        self.elevation = self.z.atan2(self.distance_2d).to_degrees();

        // Speed scalar
        self.speed = (self.vx * self.vx + self.vy * self.vy + self.vz * self.vz).sqrt();
    }

    pub fn evaluate_threat(&self) -> ThreatLevel {
        if self.status != DroneStatus::Active {
            return ThreatLevel::Low;
        }

        let range = self.distance_3d;
        
        match self.drone_type {
            DroneType::Attack => {
                if range < 600.0 {
                    ThreatLevel::Critical
                } else if range < 1500.0 {
                    ThreatLevel::High
                } else if range < 2500.0 {
                    ThreatLevel::Medium
                } else {
                    ThreatLevel::Low
                }
            }
            DroneType::Swarm => {
                if range < 500.0 {
                    ThreatLevel::Critical
                } else if range < 1200.0 {
                    ThreatLevel::High
                } else if range < 2000.0 {
                    ThreatLevel::Medium
                } else {
                    ThreatLevel::Low
                }
            }
            DroneType::Surveillance => {
                if range < 800.0 {
                    ThreatLevel::High
                } else if range < 1800.0 {
                    ThreatLevel::Medium
                } else {
                    ThreatLevel::Low
                }
            }
        }
    }
}

// Utility helper functions
fn distance_3d(x: f64, y: f64, z: f64) -> f64 {
    (x*x + y*y + z*z).sqrt()
}

fn format_type(t: DroneType) -> &'static str {
    match t {
        DroneType::Surveillance => "SURVEILLANCE",
        DroneType::Attack => "ATTACK",
        DroneType::Swarm => "SWARM",
    }
}

// A simple deterministic pseudo-random helper for simulation variability
static mut SEED: u64 = 12345;
fn rand_offset() -> f64 {
    unsafe {
        SEED = SEED.wrapping_mul(6364136223846793005).wrapping_add(1);
        let val = (SEED >> 32) as u32;
        (val as f64) / (u32::MAX as f64)
    }
}

fn rand_id() -> u32 {
    (rand_offset() * 90.0) as u32 + 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_calculations() {
        let mut drone = Drone {
            id: "TEST".to_string(),
            drone_type: DroneType::Surveillance,
            status: DroneStatus::Active,
            threat_level: ThreatLevel::Low,
            x: 3.0,
            y: 4.0,
            z: 12.0,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            distance_3d: 0.0,
            distance_2d: 0.0,
            azimuth: 0.0,
            elevation: 0.0,
            signal_strength: 1.0,
            battery: 100.0,
            size_meters: 1.0,
            speed: 0.0,
        };

        drone.update_metrics();

        assert_eq!(drone.distance_2d, 5.0);
        assert_eq!(drone.distance_3d, 13.0);
        assert_eq!(drone.azimuth, (4.0f64).atan2(3.0f64).to_degrees());
        assert_eq!(drone.elevation, (12.0f64).atan2(5.0f64).to_degrees());
    }
}
