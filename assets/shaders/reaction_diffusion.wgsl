// protean clouds: flowing volumetric noise (nimitz technique)
struct VertexOutput { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var o: VertexOutput;
    let x = f32(i32(vi)/2)*4.0-1.0; let y = f32(i32(vi)%2)*4.0-1.0;
    o.position = vec4<f32>(x,y,0,1); o.uv = vec2<f32>((x+1.0)/2.0,(1.0-y)/2.0); return o;
}
struct Uniforms {
    time:f32,res_x:f32,res_y:f32,beat_phase:f32,
    band0:f32,band1:f32,band2:f32,band3:f32,band4:f32,band5:f32,bpm:f32,onset:f32,
    sband0:f32,sband1:f32,sband2:f32,sband3:f32,sband4:f32,sband5:f32,
    energy:f32,spectral_centroid:f32,
    speed:f32,intensity:f32,zoom:f32,color_shift:f32,
    rotation_speed:f32,bass_reactivity:f32,flash_intensity:f32,brightness:f32,
    pband0:f32,pband1:f32,pband2:f32,pband3:f32,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

// 2x2 rotation for FBM octave rotation
fn m2(a: f32) -> mat2x2<f32> { let c=cos(a); let s=sin(a); return mat2x2(c,-s,s,c); }

fn hash(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p); let f = fract(p);
    let w = f * f * f * (f * (f * 6.0 - 15.0) + 10.0); // quintic
    return mix(mix(hash(i), hash(i+vec2(1,0)), w.x), mix(hash(i+vec2(0,1)), hash(i+vec2(1,1)), w.x), w.y);
}

// domain-warped FBM with per-octave rotation
fn warpfbm(p_in: vec2<f32>, t: f32) -> f32 {
    var p = p_in;
    var v = 0.0;
    var a = 0.5;
    let rot = m2(0.5 + t * 0.02);
    for (var i = 0; i < 6; i++) {
        v += a * (noise(p) - 0.5); // center around 0 for organic shapes
        p = rot * p * 2.0 + vec2<f32>(v * 0.3, -v * 0.2); // domain warp feedback
        a *= 0.55;
    }
    return v;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = (in.uv - 0.5) * 2.0; uv.x *= u.res_x / u.res_y;
    let t = u.time * u.speed * 0.3;
    let bass = u.sband0 + u.sband1;
    let mids = u.sband2 + u.sband3;
    let highs = u.sband4 + u.sband5;

    uv *= u.zoom;
    let ca = t * u.rotation_speed * 0.06;
    uv = vec2<f32>(uv.x*cos(ca)-uv.y*sin(ca), uv.x*sin(ca)+uv.y*cos(ca));

    // intensity scales frequency
    let freq = u.intensity;
    let warp_bass = bass * u.bass_reactivity * 0.15;
    let n1 = warpfbm(uv * 1.5 * freq + t * 0.1 + vec2<f32>(0.0, warp_bass), t);
    let n2 = warpfbm(uv * 2.0 * freq - t * 0.08 + vec2<f32>(n1 * 0.5, 0.0), t * 0.7);
    let n3 = warpfbm(uv * 0.8 * freq + vec2<f32>(n2, n1) * 0.3, t * 1.3);

    // deep purples, blues, magentas
    let hue = u.color_shift + n1 * 0.3 + n3 * 0.2 + t * 0.01;
    let c1 = vec3<f32>(0.6+0.4*sin(hue*6.28), 0.2+0.3*sin(hue*6.28+2.5), 0.8+0.2*sin(hue*6.28+4.0));
    let c2 = vec3<f32>(0.2+0.3*sin(hue*6.28+1.0), 0.5+0.4*sin(hue*6.28+3.5), 1.0);

    let pattern = n1 * 0.5 + 0.5;
    var col = mix(c1, c2, smoothstep(0.3, 0.7, pattern));

    // edge highlights where noise layers meet
    let edge = abs(n1 - n2) * 3.0;
    col += vec3<f32>(1.0, 0.6, 0.9) * edge * 0.3 * u.intensity;

    // darker at edges, brighter where patterns converge
    let convergence = 1.0 - abs(n3) * 1.5;
    col *= (0.4 + convergence * 0.8) * u.intensity;

    // high freq sparkle
    let sparkle = max(noise(uv * 30.0 + t * 3.0) - 0.7, 0.0) * 3.3;
    col += vec3<f32>(0.8, 0.6, 1.0) * sparkle * highs * 0.15;

    // beat pulse from center
    let r = length(uv);
    col += c1 * exp(-r * 2.0) * u.beat_phase * 0.3 * (1.0 + bass);

    col += vec3<f32>(0.9, 0.5, 1.0) * u.onset * u.flash_intensity * 0.5;

    col *= u.brightness;
    col = pow(clamp(col, vec3<f32>(0.008), vec3<f32>(1.0)), vec3<f32>(0.85));
    return vec4<f32>(col, 1.0);
}
