mod simulator;
mod web;

use std::sync::Arc;
use tokio::sync::RwLock;
use simulator::SimulationState;

#[tokio::main]
async fn main() {
    // 1. Initialize simulation state
    let state = Arc::new(RwLock::new(SimulationState::new()));

    // 2. Spawn simulation loop (runs every 100ms, updating physics by dt = 0.1 seconds)
    let sim_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            let mut lock = sim_state.write().await;
            lock.update(0.1);
        }
    });

    // 3. Configure and serve web router
    let app = web::create_router(state);
    
    // Bind to localhost port 8080
    let addr = "0.0.0.0:8080";
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[FATAL] Failed to bind to address {}: {}", addr, e);
            return;
        }
    };

    println!("======================================================================");
    println!("  AFV AI-POWERED DRONE COUNTER-MEASURES TARGETING SYSTEM ONLINE");
    println!("  Radar tracking active. Jamming emitters standing by.");
    println!("  Dashboard URL: http://localhost:8080");
    println!("======================================================================");

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("[FATAL] Server error: {}", e);
    }
}
