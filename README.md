# Flow 2.0 (Rust Edition) 🦀

<div align="center">
  <img src="assets/icon.png" width="128" alt="Flow AI Icon" />
</div>


This is the high-performance, native Rust version of **Flow**, an open-source voice dictation assistant inspired by the iPhone's Dynamic Island aesthetic. 

## Features
- **Ultra-Fast & Native**: Written in Rust, meaning zero Python overhead and minimal memory footprint.
- **Hardware-Accelerated UI**: Uses `egui` to render a true transparent, borderless Dynamic Island directly on the GPU, avoiding Windows compositor crashes.
- **Responsive Animations**: The visualizer stems use physics-based interpolation to smoothly react to your voice at 60 FPS.
- **AI-Powered Polishing**: Uses Groq API (Llama 3.3) to automatically fix grammar and remove filler words from your speech before typing it out.
- **Global Hotkey & Auto-Type**: Press `F9` anywhere to start/stop listening. The final text is automatically typed directly into your active window.

## Architecture Stack
- **UI:** `egui` via `eframe`
- **Audio Capture:** `cpal` (cross-platform audio library)
- **Automation:** `global-hotkey` for listeners and `enigo` for injecting keystrokes.
- **AI Processing:** `reqwest` for Groq API integration (with native ONNX setup stubs for local inference using Nemotron/Parakeet).

## Usage
1. Provide your Groq API key:
   Create a `.env` file in the root directory:
   ```env
   GROQ_API_KEY=your_key_here
   ```
2. Build and run the release version:
   ```bash
   cargo run --release
   ```
3. Press `F9` anywhere to summon the Dynamic Island and start dictating!
