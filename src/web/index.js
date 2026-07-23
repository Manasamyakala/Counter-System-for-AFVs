// AI-Powered Counter System for Military Vehicles (AFVs) - Client Controller

let ws;
let state = {
    afv: { name: "M1A2 ABRAMS V", x: 0, y: 0, z: 0, speed: 0, heading: 0, jammer_active: false, jammer_range: 1500, system_status: "STANDBY" },
    drones: {},
    logs: []
};

let selectedDroneId = null;
let radarSweepAngle = 0;
let dpr = window.devicePixelRatio || 1;

const MAX_RANGE_HORIZONTAL = 3000; // meters
const MAX_RANGE_VERTICAL = 500; // meters (Altitude)
const MAX_GROUND_DIST = 3000; // meters (Horizontal Ground Distance)

// DOM Elements
const radarCanvas = document.getElementById('radar-canvas');
const elevationCanvas = document.getElementById('elevation-canvas');
const targetList = document.getElementById('target-list');
const selectedTargetIdDisp = document.getElementById('selected-target-id');
const fireJammerBtn = document.getElementById('fire-jammer-btn');
const terminalLog = document.getElementById('terminal-log');
const systemTime = document.getElementById('system-time');

const afvNameDisp = document.getElementById('afv-name');
const afvCoordDisp = document.getElementById('afv-coord');
const afvSpeedDisp = document.getElementById('afv-speed');

// Canvas Contexts
const ctxRadar = radarCanvas.getContext('2d');
const ctxElevation = elevationCanvas.getContext('2d');

// Initialize WebSockets and Event Listeners
function init() {
    setupCanvases();
    window.addEventListener('resize', setupCanvases);

    // Dynamic Clock
    setInterval(() => {
        const d = new Date();
        systemTime.textContent = d.toTimeString().split(' ')[0];
    }, 1000);

    connectWebSocket();

    // Fire Button Listener
    fireJammerBtn.addEventListener('click', engageCountermeasure);

    // Animation Loop
    requestAnimationFrame(animationLoop);
}

function setupCanvases() {
    dpr = window.devicePixelRatio || 1;

    // Scale radar canvas for High-DPI screens
    const parentR = radarCanvas.parentElement;
    const wR = parentR.clientWidth;
    const hR = parentR.clientHeight;
    radarCanvas.width = wR * dpr;
    radarCanvas.height = hR * dpr;
    radarCanvas.style.width = wR + "px";
    radarCanvas.style.height = hR + "px";
    ctxRadar.setTransform(dpr, 0, 0, dpr, 0, 0);

    // Scale elevation profile canvas for High-DPI screens
    const parentE = elevationCanvas.parentElement;
    const wE = parentE.clientWidth;
    const hE = parentE.clientHeight;
    elevationCanvas.width = wE * dpr;
    elevationCanvas.height = hE * dpr;
    elevationCanvas.style.width = wE + "px";
    elevationCanvas.style.height = hE + "px";
    ctxElevation.setTransform(dpr, 0, 0, dpr, 0, 0);
}

function connectWebSocket() {
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    ws = new WebSocket(`${proto}//${host}/ws`);

    ws.onopen = () => {
        addLocalLog("[SYSTEM] WebSocket telemetry link established.");
    };

    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            state = data;
            updateUI();
        } catch (e) {
            console.error("Error parsing WebSocket state:", e);
        }
    };

    ws.onerror = (err) => {
        console.error("WebSocket error:", err);
    };

    ws.onclose = () => {
        addLocalLog("[WARNING] Telemetry link lost. Reconnecting in 3s...");
        setTimeout(connectWebSocket, 3000);
    };
}

function addLocalLog(msg) {
    const time = new Date().toTimeString().split(' ')[0];
    const logDiv = document.createElement('div');
    logDiv.textContent = `[${time}] ${msg}`;
    terminalLog.appendChild(logDiv);
    terminalLog.scrollTop = terminalLog.scrollHeight;
}

function updateUI() {
    // Header telemetry
    afvNameDisp.textContent = state.afv.name;
    afvCoordDisp.textContent = `X: ${state.afv.x.toFixed(1)}, Y: ${state.afv.y.toFixed(1)}`;
    afvSpeedDisp.textContent = `${(state.afv.speed * 3.6).toFixed(1)} km/h`;

    // Target list render
    renderTargetList();

    // Logs render
    renderLogs();
}

function renderTargetList() {
    const dronesArray = Object.values(state.drones);
    if (dronesArray.length === 0) {
        targetList.innerHTML = `<div class="no-targets">SCANNING AIRSPACE... NO ACTIVE THREATS</div>`;
        return;
    }

    // Sort by threat priority
    const priority = { 'Critical': 4, 'High': 3, 'Medium': 2, 'Low': 1 };
    dronesArray.sort((a, b) => {
        if (a.status === 'Neutralized' || a.status === 'Crashed') return 1;
        if (b.status === 'Neutralized' || b.status === 'Crashed') return -1;
        return (priority[b.threat_level] || 0) - (priority[a.threat_level] || 0);
    });

    targetList.innerHTML = dronesArray.map(drone => {
        const isSelected = drone.id === selectedDroneId;
        let badgeClass = `badge-${drone.threat_level.toLowerCase()}`;
        let statusText = drone.threat_level;
        
        if (drone.status.Jamming) {
            badgeClass = 'badge-jamming';
            statusText = `JAMMING ${(drone.status.Jamming.progress * 100).toFixed(0)}%`;
        } else if (drone.status === 'Neutralized' || drone.status === 'Crashed') {
            badgeClass = 'badge-neutralized';
            statusText = 'DOWN';
        }

        return `
            <div class="target-row ${isSelected ? 'selected' : ''}" onclick="selectDrone('${drone.id}')">
                <div class="target-row-header">
                    <span class="target-id">${drone.id}</span>
                    <span class="target-badge ${badgeClass}">${statusText}</span>
                </div>
                <div class="target-row-telemetry">
                    <span>Range: ${drone.distance_3d.toFixed(1)}m</span>
                    <span>Alt: ${drone.z.toFixed(1)}m</span>
                    <span>Speed: ${(drone.speed * 3.6).toFixed(1)} km/h</span>
                    <span>Type: ${drone.drone_type}</span>
                </div>
                <div class="target-row-math">
                    <span>D_2D: ${drone.distance_2d.toFixed(1)}m</span>
                    <span>Az: ${drone.azimuth.toFixed(1)}°</span>
                    <span>El: ${drone.elevation.toFixed(1)}°</span>
                </div>
            </div>
        `;
    }).join('');
}

function renderLogs() {
    terminalLog.innerHTML = state.logs.map(log => `<div>${log}</div>`).join('');
    terminalLog.scrollTop = terminalLog.scrollHeight;
}

window.selectDrone = function(id) {
    const drone = state.drones[id];
    if (drone && drone.status !== 'Neutralized' && drone.status !== 'Crashed' && !drone.status.Jamming) {
        selectedDroneId = id;
        selectedTargetIdDisp.textContent = id;
        fireJammerBtn.disabled = false;
    } else if (drone && drone.status.Jamming) {
        selectedDroneId = id;
        selectedTargetIdDisp.textContent = `${id} (JAMMING)`;
        fireJammerBtn.disabled = true;
    } else {
        selectedDroneId = null;
        selectedTargetIdDisp.textContent = "NONE SELECTED";
        fireJammerBtn.disabled = true;
    }
};

function engageCountermeasure() {
    if (!selectedDroneId) return;

    const jamType = document.querySelector('input[name="jam-mode"]:checked').value;
    
    fetch('/api/soft_kill', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            drone_id: selectedDroneId,
            jam_type: jamType
        })
    })
    .then(res => {
        if (res.ok) {
            addLocalLog(`[SYSTEM] Countermeasure command sent: ${jamType} against ${selectedDroneId}`);
            fireJammerBtn.disabled = true;
        } else {
            addLocalLog(`[ERROR] Failed to send countermeasure directive.`);
        }
    })
    .catch(err => {
        console.error("Error triggering soft kill:", err);
    });
}

// Animation Loop
function animationLoop(timestamp) {
    radarSweepAngle = (timestamp * 0.0006) % (2 * Math.PI); // rotation speed

    drawRadar();
    drawElevationProfile();

    requestAnimationFrame(animationLoop);
}

// DRAWING 1: TOP-VIEW RADAR
function drawRadar() {
    // Account for High-DPI backing store scaling (use logical dimensions)
    const w = radarCanvas.width / dpr;
    const h = radarCanvas.height / dpr;
    const cx = w / 2;
    const cy = h / 2;
    const rMax = Math.min(w, h) / 2 - 25;

    // Clear
    ctxRadar.clearRect(0, 0, w, h);

    // Draw Background Circles (Ranges)
    ctxRadar.strokeStyle = 'rgba(46, 77, 52, 0.4)';
    ctxRadar.lineWidth = 1;
    const ranges = [500, 1000, 1500, 2000, 2500, 3000];
    ranges.forEach(range => {
        const radius = (range / MAX_RANGE_HORIZONTAL) * rMax;
        ctxRadar.beginPath();
        ctxRadar.arc(cx, cy, radius, 0, 2 * Math.PI);
        ctxRadar.stroke();

        // Draw solid background box behind range text for high contrast readability
        ctxRadar.font = 'bold 11px "JetBrains Mono", monospace';
        const txt = `${(range/1000).toFixed(1)}km`;
        const txtW = ctxRadar.measureText(txt).width;
        
        ctxRadar.fillStyle = '#060907';
        ctxRadar.fillRect(cx + radius - txtW/2 - 4, cy - 14, txtW + 8, 12);

        ctxRadar.fillStyle = '#a3cca8';
        ctxRadar.fillText(txt, cx + radius - txtW/2, cy - 4);
    });

    // Crosshairs
    ctxRadar.strokeStyle = 'rgba(46, 77, 52, 0.5)';
    ctxRadar.lineWidth = 1;
    ctxRadar.beginPath();
    ctxRadar.moveTo(cx - rMax, cy);
    ctxRadar.lineTo(cx + rMax, cy);
    ctxRadar.moveTo(cx, cy - rMax);
    ctxRadar.lineTo(cx, cy + rMax);
    ctxRadar.stroke();

    // Radar Sweep
    const grad = ctxRadar.createRadialGradient(cx, cy, 0, cx, cy, rMax);
    grad.addColorStop(0, 'rgba(57, 255, 20, 0.06)');
    grad.addColorStop(1, 'rgba(57, 255, 20, 0.0)');
    ctxRadar.fillStyle = grad;
    
    ctxRadar.beginPath();
    ctxRadar.moveTo(cx, cy);
    ctxRadar.arc(cx, cy, rMax, radarSweepAngle - 0.25, radarSweepAngle);
    ctxRadar.closePath();
    ctxRadar.fill();

    // Faint Sweep Edge line
    ctxRadar.strokeStyle = 'rgba(57, 255, 20, 0.4)';
    ctxRadar.lineWidth = 1.5;
    ctxRadar.beginPath();
    ctxRadar.moveTo(cx, cy);
    ctxRadar.lineTo(cx + Math.cos(radarSweepAngle) * rMax, cy + Math.sin(radarSweepAngle) * rMax);
    ctxRadar.stroke();

    // AFV jammer active range visual
    if (state.afv.jammer_active) {
        ctxRadar.strokeStyle = 'rgba(0, 255, 255, 0.18)';
        ctxRadar.fillStyle = 'rgba(0, 255, 255, 0.02)';
        const jamRad = (state.afv.jammer_range / MAX_RANGE_HORIZONTAL) * rMax;
        ctxRadar.beginPath();
        ctxRadar.arc(cx, cy, jamRad, 0, 2 * Math.PI);
        ctxRadar.fill();
        ctxRadar.stroke();
    }

    // AFV tank in center
    ctxRadar.fillStyle = '#39ff14';
    ctxRadar.beginPath();
    ctxRadar.arc(cx, cy, 6, 0, 2 * Math.PI);
    ctxRadar.fill();
    ctxRadar.strokeStyle = '#ffffff';
    ctxRadar.lineWidth = 2;
    ctxRadar.stroke();

    // Draw Drones
    Object.values(state.drones).forEach(drone => {
        const dx = (drone.x / MAX_RANGE_HORIZONTAL) * rMax;
        const dy = -(drone.y / MAX_RANGE_HORIZONTAL) * rMax;

        const screenX = cx + dx;
        const screenY = cy + dy;

        // Skip if outside canvas range
        if (Math.abs(dx) > rMax || Math.abs(dy) > rMax) return;

        // Get status styles
        let color = '#39ff14';
        let pulsing = false;
        let isJamming = false;
        let isDead = false;

        if (drone.status.Jamming) {
            color = '#00ffff';
            isJamming = true;
            pulsing = true;
        } else if (drone.status === 'Neutralized' || drone.status === 'Crashed') {
            color = '#888888';
            isDead = true;
        } else {
            switch (drone.threat_level) {
                case 'Critical': color = '#ff3333'; pulsing = true; break;
                case 'High': color = '#ff9900'; pulsing = true; break;
                case 'Medium': color = '#e6e600'; break;
                case 'Low': color = '#00bfff'; break;
            }
        }

        // Draw selection ring
        if (drone.id === selectedDroneId) {
            ctxRadar.strokeStyle = '#ffffff';
            ctxRadar.lineWidth = 2;
            ctxRadar.beginPath();
            ctxRadar.arc(screenX, screenY, 15, 0, 2 * Math.PI);
            ctxRadar.stroke();
        }

        // Draw Target dot
        ctxRadar.fillStyle = color;
        ctxRadar.beginPath();
        if (isDead) {
            ctxRadar.strokeStyle = color;
            ctxRadar.lineWidth = 2.5;
            ctxRadar.moveTo(screenX - 6, screenY - 6);
            ctxRadar.lineTo(screenX + 6, screenY + 6);
            ctxRadar.moveTo(screenX - 6, screenY + 6);
            ctxRadar.lineTo(screenX + 6, screenY - 6);
            ctxRadar.stroke();
        } else {
            ctxRadar.arc(screenX, screenY, 7, 0, 2 * Math.PI);
            ctxRadar.fill();

            // Pulsing threat ring
            if (pulsing) {
                const pulseRadius = 7 + (Math.sin(Date.now() * 0.012) + 1) * 5;
                ctxRadar.strokeStyle = color;
                ctxRadar.lineWidth = 1.2;
                ctxRadar.beginPath();
                ctxRadar.arc(screenX, screenY, pulseRadius, 0, 2 * Math.PI);
                ctxRadar.stroke();
            }
        }

        // Target label text with solid background box for maximum sharpness
        ctxRadar.font = 'bold 12px "JetBrains Mono", monospace';
        const label = `${drone.id} [${drone.distance_3d.toFixed(0)}m]`;
        const labelW = ctxRadar.measureText(label).width;
        
        ctxRadar.fillStyle = '#060907';
        ctxRadar.fillRect(screenX + 11, screenY - 17, labelW + 6, 14);
        
        ctxRadar.fillStyle = color;
        ctxRadar.fillText(label, screenX + 14, screenY - 6);

        // Vector velocity line
        if (!isDead && !isJamming) {
            ctxRadar.strokeStyle = color;
            ctxRadar.lineWidth = 1.5;
            ctxRadar.beginPath();
            ctxRadar.moveTo(screenX, screenY);
            const relative_vx = (drone.vx / MAX_RANGE_HORIZONTAL) * rMax * 5;
            const relative_vy = -(drone.vy / MAX_RANGE_HORIZONTAL) * rMax * 5;
            ctxRadar.lineTo(screenX + relative_vx, screenY + relative_vy);
            ctxRadar.stroke();
        }

        // Drawing the Jammer Beam
        if (isJamming) {
            ctxRadar.strokeStyle = 'rgba(0, 255, 255, 0.6)';
            ctxRadar.lineWidth = 2.5;
            ctxRadar.setLineDash([5, 5]);
            ctxRadar.beginPath();
            ctxRadar.moveTo(cx, cy);
            ctxRadar.lineTo(screenX, screenY);
            ctxRadar.stroke();
            ctxRadar.setLineDash([]);

            // Draw interference ripples at drone
            ctxRadar.strokeStyle = '#00ffff';
            ctxRadar.lineWidth = 1.2;
            ctxRadar.beginPath();
            const rippleR = 12 + (Date.now() % 400) * 0.04;
            ctxRadar.arc(screenX, screenY, rippleR, 0, 2 * Math.PI);
            ctxRadar.stroke();
        }
    });
}

// DRAWING 2: DOWN-VIEW HEIGHT PROFILE (Altitude Cross Section)
function drawElevationProfile() {
    const w = elevationCanvas.width / dpr;
    const h = elevationCanvas.height / dpr;
    
    // Clear
    ctxElevation.clearRect(0, 0, w, h);

    // Padding settings
    const padLeft = 60;
    const padBottom = 40;
    const padRight = 20;
    const padTop = 20;

    const graphW = w - padLeft - padRight;
    const graphH = h - padTop - padBottom;

    // Draw Grid Lines (Altitude - Vertical axes)
    ctxElevation.strokeStyle = 'rgba(46, 77, 52, 0.4)';
    ctxElevation.lineWidth = 1;
    
    const altSteps = [100, 200, 300, 400, 500];
    altSteps.forEach(alt => {
        const py = padTop + graphH - (alt / MAX_RANGE_VERTICAL) * graphH;
        ctxElevation.beginPath();
        ctxElevation.moveTo(padLeft, py);
        ctxElevation.lineTo(padLeft + graphW, py);
        ctxElevation.stroke();

        // Solid backing card behind ticks
        ctxElevation.font = 'bold 12px "JetBrains Mono", monospace';
        const txt = `${alt}m`;
        const txtW = ctxElevation.measureText(txt).width;

        ctxElevation.fillStyle = '#060907';
        ctxElevation.fillRect(padLeft - txtW - 8, py - 7, txtW + 4, 13);

        ctxElevation.fillStyle = '#a3cca8';
        ctxElevation.fillText(txt, padLeft - txtW - 6, py + 3);
    });

    // Draw Grid Lines (Horizontal Ground range)
    const distSteps = [500, 1000, 1500, 2000, 2500, 3000];
    distSteps.forEach(dist => {
        const px = padLeft + (dist / MAX_GROUND_DIST) * graphW;
        ctxElevation.beginPath();
        ctxElevation.moveTo(px, padTop);
        ctxElevation.lineTo(px, padTop + graphH);
        ctxElevation.stroke();

        // Solid backing card behind ticks
        ctxElevation.font = 'bold 12px "JetBrains Mono", monospace';
        const txt = `${(dist/1000).toFixed(1)}km`;
        const txtW = ctxElevation.measureText(txt).width;

        ctxElevation.fillStyle = '#060907';
        ctxElevation.fillRect(px - txtW/2 - 3, padTop + graphH + 4, txtW + 6, 12);

        ctxElevation.fillStyle = '#a3cca8';
        ctxElevation.fillText(txt, px - txtW/2, padTop + graphH + 14);
    });

    // Draw Axis lines
    ctxElevation.strokeStyle = 'rgba(46, 77, 52, 0.7)';
    ctxElevation.lineWidth = 2;
    ctxElevation.beginPath();
    ctxElevation.moveTo(padLeft, padTop);
    ctxElevation.lineTo(padLeft, padTop + graphH);
    ctxElevation.lineTo(padLeft + graphW, padTop + graphH);
    ctxElevation.stroke();

    // Draw Jammer envelope cone
    const jamLimitX = padLeft + (1500 / MAX_GROUND_DIST) * graphW;
    ctxElevation.fillStyle = 'rgba(0, 255, 255, 0.03)';
    ctxElevation.beginPath();
    ctxElevation.moveTo(padLeft, padTop + graphH);
    ctxElevation.lineTo(jamLimitX, padTop);
    ctxElevation.lineTo(padLeft, padTop);
    ctxElevation.closePath();
    ctxElevation.fill();
    
    ctxElevation.strokeStyle = 'rgba(0, 255, 255, 0.1)';
    ctxElevation.beginPath();
    ctxElevation.moveTo(padLeft, padTop + graphH);
    ctxElevation.lineTo(jamLimitX, padTop);
    ctxElevation.stroke();

    // AFV symbol at bottom-left
    ctxElevation.fillStyle = '#39ff14';
    ctxElevation.beginPath();
    ctxElevation.arc(padLeft, padTop + graphH, 7, 0, 2 * Math.PI);
    ctxElevation.fill();
    ctxElevation.strokeStyle = '#ffffff';
    ctxElevation.lineWidth = 2;
    ctxElevation.stroke();
    
    // Label AFV text
    ctxElevation.font = 'bold 12px "JetBrains Mono", monospace';
    ctxElevation.fillStyle = '#060907';
    ctxElevation.fillRect(padLeft - 13, padTop + graphH - 24, 26, 14);
    ctxElevation.fillStyle = '#ffffff';
    ctxElevation.fillText("AFV", padLeft - 11, padTop + graphH - 13);

    // Draw Drones onto Side/Down View Profile
    Object.values(state.drones).forEach(drone => {
        const px = padLeft + (drone.distance_2d / MAX_GROUND_DIST) * graphW;
        const py = padTop + graphH - (drone.z / MAX_RANGE_VERTICAL) * graphH;

        if (drone.distance_2d > MAX_GROUND_DIST || drone.z > MAX_RANGE_VERTICAL) return;

        let color = '#39ff14';
        let isJamming = false;
        let isDead = false;

        if (drone.status.Jamming) {
            color = '#00ffff';
            isJamming = true;
        } else if (drone.status === 'Neutralized' || drone.status === 'Crashed') {
            color = '#888888';
            isDead = true;
        } else {
            switch (drone.threat_level) {
                case 'Critical': color = '#ff3333'; break;
                case 'High': color = '#ff9900'; break;
                case 'Medium': color = '#e6e600'; break;
                case 'Low': color = '#00bfff'; break;
            }
        }

        // Draw dotted height projection line to ground
        ctxElevation.strokeStyle = 'rgba(46, 77, 52, 0.6)';
        ctxElevation.setLineDash([2, 4]);
        ctxElevation.beginPath();
        ctxElevation.moveTo(px, py);
        ctxElevation.lineTo(px, padTop + graphH);
        ctxElevation.stroke();
        ctxElevation.setLineDash([]);

        // Draw drone node indicator
        ctxElevation.fillStyle = color;
        ctxElevation.beginPath();
        if (isDead) {
            ctxElevation.strokeStyle = color;
            ctxElevation.lineWidth = 2.0;
            ctxElevation.moveTo(px - 5, py - 5);
            ctxElevation.lineTo(px + 5, py + 5);
            ctxElevation.moveTo(px - 5, py + 5);
            ctxElevation.lineTo(px + 5, py - 5);
            ctxElevation.stroke();
        } else if (drone.drone_type === 'Attack') {
            // Triangle pointing down
            ctxElevation.moveTo(px, py + 6);
            ctxElevation.lineTo(px - 6, py - 6);
            ctxElevation.lineTo(px + 6, py - 6);
            ctxElevation.closePath();
            ctxElevation.fill();
        } else {
            // Circle
            ctxElevation.arc(px, py, 5.5, 0, 2 * Math.PI);
            ctxElevation.fill();
        }

        // Draw selection ring
        if (drone.id === selectedDroneId) {
            ctxElevation.strokeStyle = '#ffffff';
            ctxElevation.lineWidth = 1.8;
            ctxElevation.beginPath();
            ctxElevation.arc(px, py, 12, 0, 2 * Math.PI);
            ctxElevation.stroke();
        }

        // Label with background card
        ctxElevation.font = 'bold 11px "JetBrains Mono", monospace';
        const label = `${drone.id} [Z:${drone.z.toFixed(0)}m]`;
        const labelW = ctxElevation.measureText(label).width;

        ctxElevation.fillStyle = '#060907';
        ctxElevation.fillRect(px + 8, py - 14, labelW + 4, 13);

        ctxElevation.fillStyle = color;
        ctxElevation.fillText(label, px + 10, py - 3);

        // Jammer beam source is bottom-left (AFV coordinates)
        if (isJamming) {
            ctxElevation.strokeStyle = 'rgba(0, 255, 255, 0.4)';
            ctxElevation.lineWidth = 2;
            ctxElevation.setLineDash([3, 3]);
            ctxElevation.beginPath();
            ctxElevation.moveTo(padLeft, padTop + graphH);
            ctxElevation.lineTo(px, py);
            ctxElevation.stroke();
            ctxElevation.setLineDash([]);
        }
    });
}

// Start
document.addEventListener('DOMContentLoaded', init);
