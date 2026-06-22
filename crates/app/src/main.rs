use std::path::PathBuf;

use core::bus::PluginBus;
use core::config::MyceliumConfig;
use core::engine::{EngineParams, ShaderSource};
use shaders::registry::ShaderRegistry;
use tracing_subscriber::EnvFilter;

fn print_help() {
    eprintln!("Mycelium - Audio-Reactive Visual Engine");
    eprintln!();
    eprintln!("USAGE: mycelium [OPTIONS]");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("  --config <PATH>   Config file (default: config/default.toml)");
    eprintln!("  --shader <NAME>   Start with this shader (e.g. fractal, voronoi)");
    eprintln!("  --fullscreen      Start in fullscreen mode");
    eprintln!("  --mic             Use microphone instead of loopback");
    eprintln!("  --list-shaders    List available shaders and exit");
    eprintln!("  --help            Show this help");
    eprintln!();
    eprintln!("CONTROLS:");
    eprintln!("  Tab        Toggle GUI overlay");
    eprintln!("  F11        Toggle fullscreen");
    eprintln!("  1-0        Switch shader");
    eprintln!("  Escape     Quit");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut config_path = PathBuf::from("config/default.toml");
    let mut start_shader: Option<String> = None;
    let mut force_fullscreen = false;
    let mut force_mic = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--config" => {
                i += 1;
                if i < args.len() {
                    config_path = PathBuf::from(&args[i]);
                }
            }
            "--shader" => {
                i += 1;
                if i < args.len() {
                    start_shader = Some(args[i].clone());
                }
            }
            "--fullscreen" => force_fullscreen = true,
            "--mic" => force_mic = true,
            "--list-shaders" => {
                let reg = ShaderRegistry::default();
                for name in reg.names() {
                    let entry = reg.get(name).unwrap();
                    println!("{:15} - {}", name, entry.description);
                }
                return;
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_help();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("mycelium=info".parse().unwrap()),
        )
        .init();

    let mut config = MyceliumConfig::load(&config_path).unwrap_or_else(|e| {
        tracing::warn!("Failed to load config: {e}, using defaults");
        MyceliumConfig::default()
    });

    if force_fullscreen {
        config.window.fullscreen = true;
    }
    if force_mic {
        config.audio.input = core::config::AudioInput::Mic;
    }

    tracing::info!("Starting Mycelium in {:?} mode", config.mode);

    let registry = ShaderRegistry::default();
    let shaders: Vec<ShaderSource> = registry
        .names()
        .iter()
        .map(|name| {
            let entry = registry.get(name).expect("Registry inconsistency");
            ShaderSource {
                name: entry.name.clone(),
                wgsl: entry.source.clone(),
                path: Some(entry.path.clone()),
            }
        })
        .collect();

    tracing::info!(
        "Loaded {} shaders: {}",
        shaders.len(),
        registry.names().join(", ")
    );

    let bus = PluginBus::new();
    let mut keep_alive: Vec<Box<dyn std::any::Any + Send>> = Vec::new();

    match audio::AudioEngine::start(
        config.audio.input,
        config.audio.fft_size,
        config.audio.processing.clone(),
        bus.audio_buffer(),
    ) {
        Ok(engine) => {
            tracing::info!("Audio engine started");
            keep_alive.push(Box::new(engine));
        }
        Err(e) => {
            tracing::warn!("Audio engine failed to start: {e}. Running without audio.");
        }
    }

    let midi = io::midi::MidiHandler::start(bus.event_sender());
    keep_alive.push(Box::new(midi));

    // osc input on port 9000
    let osc = io::osc::OscHandler::start(9000, bus.event_sender());
    keep_alive.push(Box::new(osc));

    let gamepad = io::gamepad::GamepadHandler::start(bus.event_sender());
    keep_alive.push(Box::new(gamepad));

    let script: Option<Box<dyn core::engine::ScriptEvaluator>> =
        match script::LuaScriptEvaluator::from_file("assets/scripts/default.lua") {
            Ok(eval) => {
                tracing::info!("Lua script loaded: assets/scripts/default.lua");
                Some(Box::new(eval))
            }
            Err(e) => {
                tracing::warn!("No Lua script loaded: {e}");
                None
            }
        };

    let params = EngineParams {
        config,
        shaders,
        bus,
        script,
        _keep_alive: keep_alive,
        start_shader,
    };

    if let Err(e) = core::engine::run(params) {
        tracing::error!("Engine error: {e}");
        std::process::exit(1);
    }
}
