# Mycelium

Audio in, trippy visuals out. It grabs your system audio (or a mic), runs it through an FFT, and throws the result at a pile of GPU shaders. Everything pulses to the beat.

Rust + wgpu under the hood, so it runs on Vulkan, Metal and DX12. Plug in a MIDI controller, send it OSC, wave a gamepad around, or script it in Lua.

![liquid shader doing its thing](assets/screenshot.png)

## Try it

```bash
cargo run -p app
```

Hit Tab for the overlay, mash 1-0 to flip through shaders. That's the whole tutorial.

## What's in the box

- 10 WGSL shaders (fractal, voronoi, kaleidoscope, reaction diffusion, strobe, and friends)
- real-time FFT: 6 frequency bands, BPM, onset detection, spectral stuff
- edit a shader on disk and it hot-reloads, no restart
- audio drives shader params, smoothed so it doesn't jitter
- MIDI CC if you want knobs to turn
- OSC on port 9000
- gamepads via gilrs
- Lua scripting when you want custom param logic
- save/load presets
- TOML config that also hot-reloads

## Keys

| Key | Does |
|-----|--------|
| Tab | overlay on/off |
| F11 | fullscreen |
| 1-9, 0 | pick a shader |
| Escape | bail |

MIDI controllers map CC messages onto shader params. OSC goes to `/cc/N` or `/trigger/N` on port 9000.

## How it's laid out

6 crates, each does one thing:

| Crate | Job |
|-------|---------|
| core | GPU context, plugin bus, config, engine loop, mapping, presets |
| audio | capture, FFT, band energy, BPM, onsets |
| shaders | shader registry + effect presets |
| script | Lua scripting |
| io | MIDI, OSC, gamepad |
| app | the binary that wires it all together |

## Building

```bash
cargo build                    # everything
cargo test --workspace         # tests
cargo clippy --workspace       # lint
```

## Config

`config/default.toml` has window size, audio input mode, FFT size, and the processing knobs. Save it and the running app picks it up.

## License

MIT
