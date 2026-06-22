// cellular: animated voronoi tessellation with second-order distance,
// domain warping, audio-driven cell colors, and smooth blending.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vertex_index) / 2) * 4.0 - 1.0;
    let y = f32(i32(vertex_index) % 2) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) / 2.0, (1.0 - y) / 2.0);
    return out;
}

struct Uniforms {
    time: f32,
    res_x: f32,
    res_y: f32,
    beat_phase: f32,
    band0: f32, band1: f32, band2: f32, band3: f32, band4: f32, band5: f32,
    bpm: f32,
    onset: f32,
    sband0: f32, sband1: f32, sband2: f32, sband3: f32, sband4: f32, sband5: f32,
    energy: f32,
    spectral_centroid: f32,
    speed: f32,
    intensity: f32,
    zoom: f32,
    color_shift: f32,
    rotation_speed: f32,
    bass_reactivity: f32,
    flash_intensity: f32,
    brightness: f32,
    pband0: f32, pband1: f32, pband2: f32, pband3: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

const TAU: f32 = 6.28318530718;

// hash for cell point positions
fn hash22(p: vec2<f32>) -> vec2<f32> {
    let k = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(k) * 43758.5453123);
}

fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn rot2(a: f32) -> mat2x2<f32> {
    let ca = cos(a);
    let sa = sin(a);
    return mat2x2<f32>(ca, -sa, sa, ca);
}

fn cosine_palette(t: f32, shift: f32) -> vec3<f32> {
    let a = vec3<f32>(0.5, 0.5, 0.5);
    let b = vec3<f32>(0.5, 0.5, 0.5);
    let c = vec3<f32>(1.0, 1.0, 1.0);
    let d = vec3<f32>(
        0.0 + shift,
        0.33 + shift * 0.7,
        0.67 + shift * 0.4
    );
    return a + b * cos(TAU * (c * t + d));
}

// smooth minimum for blending voronoi cells
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

// gradient noise for domain warping
fn gnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let w = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}

fn fbm(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var v = 0.0;
    var a = 0.5;
    for (var i = 0; i < 3; i++) {
        v += a * gnoise(p);
        p *= 2.03;
        a *= 0.5;
    }
    return v;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = (in.uv - 0.5) * 2.0;
    let aspect = u.res_x / u.res_y;
    uv.x *= aspect;

    let bass = u.sband0 + u.sband1;
    let mids = u.sband2 + u.sband3;
    let highs = u.sband4 + u.sband5;
    let t = u.time * u.speed * 0.4;

    // direct UV scale; slider is already logarithmic
    uv *= u.zoom;

    uv = rot2(t * u.rotation_speed * 0.3) * uv;

    let scale = 3.0 + u.onset * u.flash_intensity * 2.0;
    uv *= scale;

    // displace cell space with smoothed bass
    let warp_amount = bass * u.bass_reactivity * 0.5;
    let warp_x = fbm(uv * 0.7 + vec2<f32>(t * 0.1, 0.0));
    let warp_y = fbm(uv * 0.7 + vec2<f32>(0.0, t * 0.1) + 3.7);
    uv += vec2<f32>(warp_x, warp_y) * warp_amount;

    // voronoi with 9-cell neighborhood search
    let cell_p = floor(uv);
    let cell_f = fract(uv);

    var d1 = 10.0;   // nearest distance
    var d2 = 10.0;   // second nearest distance
    var nearest_cell = vec2<f32>(0.0);
    var nearest_hash = 0.0;

    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let neighbor = vec2<f32>(f32(i), f32(j));
            let cell_id = cell_p + neighbor;

            // animated cell center
            var point = hash22(cell_id);
            let cell_hash = hash21(cell_id);

            // cells pulse with beat_phase
            let pulse = sin(t + cell_hash * TAU + u.beat_phase * TAU) * 0.3;
            point = 0.5 + 0.4 * sin(t * 0.5 + point * TAU + pulse);

            let diff = neighbor + point - cell_f;
            let d = length(diff);

            // track nearest and second nearest for smooth min
            if d < d1 {
                d2 = d1;
                d1 = d;
                nearest_cell = cell_id;
                nearest_hash = cell_hash;
            } else if d < d2 {
                d2 = d;
            }
        }
    }

    let blend_k = 0.15 + mids * 0.1;
    let smooth_d = smin(d1, d2, blend_k);

    // edge detection from second-order voronoi distance
    let edge = d2 - d1;
    let edge_line = smoothstep(0.0, 0.05 + highs * 0.03, edge);

    // cell colors driven by different audio bands
    let band_selector = fract(nearest_hash * 6.0);
    var cell_audio = 0.0;
    if band_selector < 0.167 {
        cell_audio = u.sband0;
    } else if band_selector < 0.333 {
        cell_audio = u.sband1;
    } else if band_selector < 0.5 {
        cell_audio = u.sband2;
    } else if band_selector < 0.667 {
        cell_audio = u.sband3;
    } else if band_selector < 0.833 {
        cell_audio = u.sband4;
    } else {
        cell_audio = u.sband5;
    }

    let cell_color = cosine_palette(
        nearest_hash + cell_audio * 0.5 + t * 0.05,
        u.color_shift
    );

    let interior = 1.0 - smooth_d * 1.5;
    let cell_brightness = clamp(interior + cell_audio * 0.6, 0.0, 1.5);

    var color = cell_color * cell_brightness * u.intensity;

    // edge glow
    let edge_color = cosine_palette(
        d2 * 0.5 + t * 0.1 + u.spectral_centroid * 0.3,
        u.color_shift + 0.5
    );
    color = mix(color, edge_color * 1.5, (1.0 - edge_line) * 0.7);

    // onset adds a layer of smaller voronoi (cell subdivision)
    let sub_scale = 2.0;
    let sub_uv = uv * sub_scale;
    let sub_cell_p = floor(sub_uv);
    let sub_cell_f = fract(sub_uv);
    var sub_d1 = 10.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let neighbor = vec2<f32>(f32(i), f32(j));
            let cell_id = sub_cell_p + neighbor;
            var point = hash22(cell_id + 100.0);
            point = 0.5 + 0.4 * sin(t * 0.8 + point * TAU);
            let diff = neighbor + point - sub_cell_f;
            sub_d1 = min(sub_d1, length(diff));
        }
    }
    let sub_edge = smoothstep(0.0, 0.04, sub_d1);
    let sub_blend = u.onset * u.flash_intensity;
    color = mix(color, color * sub_edge * 1.5, sub_blend);

    color *= 1.0 + u.beat_phase * 0.2;

    // energy modulates overall saturation
    let grey = vec3<f32>(dot(color, vec3<f32>(0.299, 0.587, 0.114)));
    color = mix(grey, color, 0.6 + u.energy * 0.5);

    // vignette
    let vignette = 1.0 - pow(length(in.uv - 0.5) * 1.4, 2.5);
    color *= max(vignette, 0.1);

    color *= u.brightness;

    return vec4<f32>(max(color, vec3<f32>(0.0)), 1.0);
}
