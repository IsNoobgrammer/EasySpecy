<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'

// --- 1. Background Wave Canvas ---
let canvas = null
let ctx = null
let animationFrameId = null
let waves = []

const initCanvas = () => {
  canvas = document.getElementById('wave-canvas')
  if (!canvas) return
  ctx = canvas.getContext('2d')
  
  const resize = () => {
    if (!canvas) return
    canvas.width = canvas.parentElement.clientWidth || window.innerWidth
    canvas.height = canvas.parentElement.clientHeight || window.innerHeight
  }
  window.addEventListener('resize', resize)
  resize()

  // Define waves with amplitude, frequency (length), speed, phase, and base height ratio
  waves = [
    { y: 0.25, length: 0.004, amplitude: 50, speed: 0.005, phase: 0, colorDark: 'rgba(0, 232, 138, 0.06)', colorLight: 'rgba(49, 99, 66, 0.04)' },
    { y: 0.40, length: 0.003, amplitude: 70, speed: -0.004, phase: 2, colorDark: 'rgba(192, 193, 255, 0.03)', colorLight: 'rgba(130, 168, 142, 0.03)' },
    { y: 0.60, length: 0.002, amplitude: 40, speed: 0.003, phase: 4, colorDark: 'rgba(0, 232, 138, 0.04)', colorLight: 'rgba(49, 99, 66, 0.02)' }
  ]

  const animate = () => {
    if (!canvas || !ctx) return
    ctx.clearRect(0, 0, canvas.width, canvas.height)
    
    // Check if the page is currently in dark mode
    const isDark = document.documentElement.classList.contains('dark')

    waves.forEach(wave => {
      ctx.beginPath()
      ctx.strokeStyle = isDark ? wave.colorDark : wave.colorLight
      ctx.lineWidth = 1.5
      
      const baseHeight = canvas.height * wave.y
      
      for (let x = 0; x < canvas.width; x++) {
        // Drifting sine wave equation
        const y = baseHeight + Math.sin(x * wave.length + wave.phase) * wave.amplitude * Math.cos(x * 0.0005)
        if (x === 0) ctx.moveTo(x, y)
        else ctx.lineTo(x, y)
      }
      
      ctx.stroke()
      wave.phase += wave.speed
    })
    
    animationFrameId = requestAnimationFrame(animate)
  }

  animate()

  onUnmounted(() => {
    window.removeEventListener('resize', resize)
    if (animationFrameId) cancelAnimationFrame(animationFrameId)
  })
}

// --- 2. Live Recording Timer ---
const timeString = ref('00:00:00.00')
let timerInterval = null
let startTime = 0

const startTimer = () => {
  startTime = Date.now()
  timerInterval = setInterval(() => {
    const delta = Date.now() - startTime
    
    const hrs = Math.floor(delta / 3600000)
    const mins = Math.floor((delta % 3600000) / 60000)
    const secs = Math.floor((delta % 60000) / 1000)
    const ms = Math.floor((delta % 1000) / 10)
    
    const pad = (num, len = 2) => String(num).padStart(len, '0')
    timeString.value = `${pad(hrs)}:${pad(mins)}:${pad(secs)}.${pad(ms)}`
  }, 33)
}

// --- 3. Dynamic Spectral Waveform ---
const barCount = 36
const waveformHeights = ref(Array(barCount).fill(15))
let waveformInterval = null

const startWaveformAnimation = () => {
  waveformInterval = setInterval(() => {
    waveformHeights.value = waveformHeights.value.map(() => {
      return Math.floor(Math.random() * 75) + 10 // Height between 10% and 85%
    })
  }, 120)
}

// --- 4. Interactive Webcam Bento Card ---
const webcamOpacity = ref(85)
const webcamRadius = ref(70)

const webcamStyle = computed(() => {
  return {
    opacity: webcamOpacity.value / 100,
    transform: `scale(${0.5 + (webcamRadius.value / 100) * 0.6})`
  }
})

// --- 5. Interactive Easing Point Hover ---
const activeBezierPoint = ref(null)

// --- Lifecycle ---
onMounted(() => {
  setTimeout(() => {
    initCanvas()
    startTimer()
    startWaveformAnimation()
  }, 100)
})

onUnmounted(() => {
  if (timerInterval) clearInterval(timerInterval)
  if (waveformInterval) clearInterval(waveformInterval)
})
</script>

<template>
  <div class="custom-landing-page">
    <!-- Drift Wave Background -->
    <canvas id="wave-canvas" class="wave-bg"></canvas>

    <!-- 1. HERO SECTION -->
    <section class="custom-hero">
      <div class="hero-container">
        <!-- Hero Copy -->
        <div class="hero-left">
          <div class="brand-badge">
            <span class="badge-dot"></span>
            <span class="badge-text">v0.2.0 Release</span>
          </div>
          
          <h1 class="hero-title">
            Record like a <span class="gradient-accent">pro.</span><br />
            Pay like it's <span class="muted-title">2005.</span>
          </h1>
          
          <p class="hero-description">
            The industry standard for technical screen recording. Capture 1080p video, real-time metadata, and crystal-clear webcam overlays without the subscription bloat.
          </p>
          
          <div class="hero-ctas">
            <a class="cta-button primary" href="https://github.com/IsNoobgrammer/EasySpecy/releases" target="_blank" rel="noopener">
              <svg class="cta-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              </svg>
              Download App
            </a>
            <a class="cta-button secondary" href="/EasySpecy/guide/introduction">
              <svg class="cta-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <circle cx="12" cy="12" r="10" />
                <circle cx="12" cy="12" r="3" fill="currentColor" />
              </svg>
              Read Guide
            </a>
            <a class="cta-button secondary" href="https://github.com/IsNoobgrammer/EasySpecy" target="_blank" rel="noopener">
              <svg class="cta-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22" />
              </svg>
              GitHub
            </a>
          </div>

          <div class="tech-tags">
            <span class="tag-label">POWERED BY</span>
            <div class="tags-row">
              <span class="tech-tag">Rust</span>
              <span class="tech-tag">Tauri v2</span>
              <span class="tech-tag">Vue 3</span>
              <span class="tech-tag">WebAssembly</span>
            </div>
          </div>
        </div>

        <!-- Glassmorphic App Mockup -->
        <div class="hero-right">
          <div class="relative-wrapper">
            <!-- Background Radial Glow -->
            <div class="absolute-glow"></div>
            
            <!-- App mockup panel -->
            <div class="mock-recorder-panel">
              <!-- Window header -->
              <div class="panel-header">
                <div class="window-controls">
                  <span class="dot red"></span>
                  <span class="dot yellow"></span>
                  <span class="dot green"></span>
                </div>
                <div class="panel-title">REC_SESSION_082.mp4</div>
                <div class="spacer"></div>
              </div>
              
              <!-- Window Body -->
              <div class="panel-body">
                <div class="rec-meta-row">
                  <div class="rec-status-indicator">
                    <span class="pulse-dot"></span>
                    <span class="status-label">REC</span>
                    <span class="timer-value">{{ timeString }}</span>
                  </div>
                  <div class="format-chips">
                    <span class="format-chip">1080p</span>
                    <span class="format-chip">60 FPS</span>
                  </div>
                </div>

                <!-- Simulated Captured Area -->
                <div class="capture-preview-screen">
                  <!-- Blurred IDE content -->
                  <div class="preview-backdrop-code">
                    <pre><code><span class="keyword">const</span> EasySpecy = {
  zoom: <span class="string">"cinematic-bezier"</span>,
  audio: <span class="string">"dual-channel-1080p"</span>,
  overlay: <span class="string">"webcam-pip"</span>,
  performance: <span class="string">"rust-tauri-native"</span>
};

<span class="keyword">function</span> <span class="function">recordPro</span>() {
  EasySpecy.<span class="function">startCapture</span>();
}</code></pre>
                  </div>

                  <!-- Waveform Overlay at the bottom -->
                  <div class="waveform-analyzer-container">
                    <div 
                      v-for="(height, index) in waveformHeights" 
                      :key="index"
                      class="waveform-bar"
                      :style="{ 
                        height: height + '%',
                        backgroundColor: height > 65 ? 'var(--vp-c-brand)' : 'rgba(0, 232, 138, 0.35)'
                      }"
                    ></div>
                  </div>

                  <!-- Webcam Picture-in-Picture circle overlay -->
                  <div class="webcam-pip-circle">
                    <div class="webcam-inner">
                      <svg class="avatar-svg" viewBox="0 0 100 100" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="50" cy="35" r="15" />
                        <path d="M25 80c0-15 10-22 25-22s25 7 25 22" />
                      </svg>
                      <div class="webcam-tag">LIVE</div>
                    </div>
                  </div>
                </div>

                <!-- Live Level Sliders -->
                <div class="live-controls-grid">
                  <div class="control-gauge">
                    <div class="gauge-label">
                      <span>AUDIO INPUT GAIN</span>
                      <span class="gauge-val">82%</span>
                    </div>
                    <div class="gauge-track">
                      <div class="gauge-fill" style="width: 82%"></div>
                      <div class="gauge-thumb" style="left: 82%"></div>
                    </div>
                  </div>
                  <div class="control-gauge">
                    <div class="gauge-label">
                      <span>SYSTEM MONITOR</span>
                      <span class="gauge-val">ACTIVE</span>
                    </div>
                    <div class="monitoring-leds">
                      <span class="led active"></span>
                      <span class="led active"></span>
                      <span class="led active"></span>
                      <span class="led active"></span>
                      <span class="led"></span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 2. BENTO grid feature showcase -->
    <section class="custom-features">
      <div class="features-header">
        <span class="features-badge">Engineering Showcase</span>
        <h2 class="features-section-title">
          Precision tools for the next generation of <span class="gradient-accent">power creators.</span>
        </h2>
      </div>

      <div class="bento-grid">
        <!-- Card 1: Cinematic Auto-Zoom (2/3 width) -->
        <div class="bento-card col-span-2 relative-group">
          <div class="hover-glow-backdrop"></div>
          <div class="card-inner-content">
            <div class="card-header">
              <div class="card-icon-wrapper">
                <svg class="card-icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="11" cy="11" r="8" />
                  <line x1="21" y1="21" x2="16.65" y2="16.65" />
                  <line x1="11" y1="8" x2="11" y2="14" />
                  <line x1="8" y1="11" x2="14" y2="11" />
                </svg>
              </div>
              <div class="card-text">
                <h3 class="card-title">Cinematic Auto-Zoom</h3>
                <p class="card-details">Intelligent cursor tracking with smooth bezier motion easing curves.</p>
              </div>
            </div>

            <!-- Easing Curve visualization -->
            <div class="bento-visual curve-viz">
              <svg class="bezier-svg-grid" viewBox="0 0 400 120" fill="none">
                <!-- Grid background -->
                <line x1="0" y1="30" x2="400" y2="30" stroke="rgba(255,255,255,0.03)" />
                <line x1="0" y1="60" x2="400" y2="60" stroke="rgba(255,255,255,0.03)" />
                <line x1="0" y1="90" x2="400" y2="90" stroke="rgba(255,255,255,0.03)" />
                
                <!-- Main bezier path -->
                <path 
                  class="glowing-bezier-line" 
                  d="M 10 100 Q 120 100 200 60 T 390 20" 
                  stroke="var(--vp-c-brand)" 
                  stroke-width="3" 
                />
                
                <!-- Points -->
                <circle 
                  class="bezier-point" 
                  cx="200" 
                  cy="60" 
                  r="4.5" 
                  fill="var(--vp-c-brand)"
                  @mouseenter="activeBezierPoint = 1"
                  @mouseleave="activeBezierPoint = null"
                  :style="{ transform: activeBezierPoint === 1 ? 'scale(1.5)' : 'scale(1)' }"
                />
                <circle 
                  class="bezier-point" 
                  cx="390" 
                  cy="20" 
                  r="4.5" 
                  fill="var(--vp-c-brand)"
                  @mouseenter="activeBezierPoint = 2"
                  @mouseleave="activeBezierPoint = null"
                  :style="{ transform: activeBezierPoint === 2 ? 'scale(1.5)' : 'scale(1)' }"
                />
                
                <text x="15" y="25" class="svg-label">EASE-IN-OUT</text>
                <text x="290" y="112" class="svg-label">DURATION: 450ms</text>
              </svg>
            </div>
          </div>
        </div>

        <!-- Card 2: Keyboard Overlay (1/3 width) -->
        <div class="bento-card col-span-1 relative-group">
          <div class="hover-glow-backdrop"></div>
          <div class="card-inner-content">
            <div class="card-header">
              <div class="card-icon-wrapper">
                <svg class="card-icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="2" y="4" width="20" height="16" rx="2" ry="2" />
                  <line x1="6" y1="8" x2="6" y2="8" />
                  <line x1="10" y1="8" x2="10" y2="8" />
                  <line x1="14" y1="8" x2="14" y2="8" />
                  <line x1="18" y1="8" x2="18" y2="8" />
                  <line x1="6" y1="12" x2="6" y2="12" />
                  <line x1="18" y1="12" x2="18" y2="12" />
                  <line x1="7" y1="16" x2="17" y2="16" />
                  <line x1="10" y1="12" x2="14" y2="12" />
                </svg>
              </div>
              <div class="card-text">
                <h3 class="card-title">Keyboard Overlay</h3>
                <p class="card-details">Real-time global shortcut key capture bubbles.</p>
              </div>
            </div>

            <!-- Keyboard bubble visuals -->
            <div class="bento-visual keyboard-viz">
              <div class="keycaps-row">
                <div class="keycap">Ctrl</div>
                <div class="keycap">Shift</div>
                <div class="keycap active">Z</div>
              </div>
              <span class="latency-pill">LATENCY: 2ms</span>
            </div>
          </div>
        </div>

        <!-- Card 3: Cursor Trails (1/3 width) -->
        <div class="bento-card col-span-1 relative-group">
          <div class="hover-glow-backdrop"></div>
          <div class="card-inner-content">
            <div class="card-header">
              <div class="card-icon-wrapper">
                <svg class="card-icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M22 2L11 13" />
                  <polygon points="22 2 15 22 11 13 2 9 22 2" />
                </svg>
              </div>
              <div class="card-text">
                <h3 class="card-title">Cursor Trails</h3>
                <p class="card-details">Smooth vector particle trail paths.</p>
              </div>
            </div>

            <!-- Vector Spline preview -->
            <div class="bento-visual trail-viz">
              <svg class="trail-svg-box" viewBox="0 0 120 100" fill="none">
                <path d="M 10 80 C 30 30, 80 50, 110 15" stroke="var(--vp-c-brand)" stroke-width="2" stroke-dasharray="3,3" />
                <circle cx="10" cy="80" r="1.5" fill="var(--vp-c-brand)" opacity="0.4" />
                <circle cx="45" cy="40" r="2" fill="var(--vp-c-brand)" opacity="0.7" />
                <circle cx="75" cy="45" r="2.5" fill="var(--vp-c-brand)" />
                <polygon points="110,15 112,23 105,20" fill="var(--vp-c-text-1)" />
              </svg>
              <span class="particle-count-tag">PARTICLE COUNT: 128</span>
            </div>
          </div>
        </div>

        <!-- Card 4: Pro Webcam PiP (2/3 width) -->
        <div class="bento-card col-span-2 relative-group">
          <div class="hover-glow-backdrop"></div>
          <div class="card-inner-content">
            <div class="card-header">
              <div class="card-icon-wrapper">
                <svg class="card-icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M23 7l-7 5 7 5V7z" />
                  <rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
                </svg>
              </div>
              <div class="card-text">
                <h3 class="card-title">Pro Webcam PiP</h3>
                <p class="card-details">Bespoke circular webcam overlay with real-time controls.</p>
              </div>
            </div>

            <!-- Webcam interactive visualizers -->
            <div class="bento-visual webcam-viz-panel">
              <!-- Live Preview Adjuster -->
              <div class="webcam-preview-frame">
                <div class="live-status-dot-row">
                  <span class="status-indicator-dot"></span>
                  <span class="status-indicator-text">LIVE</span>
                </div>
                <div class="avatar-webcam-mock" :style="webcamStyle">
                  <svg class="user-silhouette-svg" viewBox="0 0 100 100" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="50" cy="35" r="16" />
                    <path d="M22 82c0-16 11-24 28-24s28 8 28 24" />
                  </svg>
                </div>
              </div>

              <!-- Opacity & Radius Sliders -->
              <div class="webcam-mixers">
                <div class="mixer-slider-group">
                  <div class="mixer-label-row">
                    <span>OPACITY</span>
                    <span class="mixer-value">{{ webcamOpacity }}%</span>
                  </div>
                  <input 
                    type="range" 
                    min="10" 
                    max="100" 
                    v-model="webcamOpacity"
                    class="mixer-input-range"
                  />
                </div>
                <div class="mixer-slider-group">
                  <div class="mixer-label-row">
                    <span>ZOOM / RADIUS</span>
                    <span class="mixer-value">{{ webcamRadius }}%</span>
                  </div>
                  <input 
                    type="range" 
                    min="10" 
                    max="100" 
                    v-model="webcamRadius"
                    class="mixer-input-range"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Card 5: Dual Audio Noise Gate (3/3 width) -->
        <div class="bento-card col-span-3 relative-group">
          <div class="hover-glow-backdrop"></div>
          <div class="card-inner-content">
            <div class="card-header">
              <div class="card-icon-wrapper">
                <svg class="card-icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3z" />
                  <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                  <line x1="12" y1="19" x2="12" y2="22" />
                </svg>
              </div>
              <div class="card-text">
                <h3 class="card-title">Dual Audio Noise Gate</h3>
                <p class="card-details">Simultaneous high-quality microphone and system audio capture with low-latency software level gates.</p>
              </div>
            </div>

            <!-- Audio Level Monitors -->
            <div class="bento-visual noise-gate-monitor">
              <div class="gate-monitor-grid">
                <div class="audio-source-status">
                  <div class="source-info">
                    <span class="source-name">MIC LEVEL</span>
                    <span class="source-db">-12.4 dB</span>
                  </div>
                  <div class="level-indicator-bar-wrapper">
                    <div class="level-indicator-bar fill-mic"></div>
                  </div>
                </div>

                <div class="audio-source-status">
                  <div class="source-info">
                    <span class="source-name">SYSTEM LEVEL</span>
                    <span class="source-db">-6.2 dB</span>
                  </div>
                  <div class="level-indicator-bar-wrapper">
                    <div class="level-indicator-bar fill-sys"></div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 3. FOOTER SECTION -->
    <footer class="custom-footer">
      <div class="footer-glow"></div>
      <div class="footer-content">
        <div class="footer-left">
          <span class="footer-brand">EasySpecy</span>
          <span class="footer-license">Released under the MIT License.</span>
          <span class="footer-copy">Copyright © 2026-present Shaurya Sharthak. All rights reserved.</span>
        </div>
        <div class="footer-right">
          <a href="/EasySpecy/guide/introduction">Documentation</a>
          <a href="/EasySpecy/config/overview">Configuration</a>
          <a href="https://github.com/IsNoobgrammer/EasySpecy" target="_blank" rel="noopener">GitHub</a>
        </div>
      </div>
    </footer>
  </div>
</template>
