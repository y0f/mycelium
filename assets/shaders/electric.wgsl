// electric: lightning bolts and plasma arcs
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

fn hash(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p); let f = fract(p); let w = f*f*(3.0-2.0*f);
    return mix(mix(hash(i), hash(i+vec2(1,0)), w.x), mix(hash(i+vec2(0,1)), hash(i+vec2(1,1)), w.x), w.y);
}

// ridged noise for lightning bolt shapes
fn ridged(p: vec2<f32>) -> f32 {
    return 1.0 - abs(noise(p) * 2.0 - 1.0);
}

fn lightning(p: vec2<f32>, t: f32, freq: f32) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var pos = p;
    for (var i = 0; i < 4; i++) {
        v += amp * ridged(pos * freq + t * (1.0 + f32(i) * 0.3));
        pos *= 2.1;
        amp *= 0.5;
    }
    return v;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = (in.uv - 0.5) * 2.0; uv.x *= u.res_x / u.res_y;
    let t = u.time * u.speed;
    let bass = u.sband0 + u.sband1;
    let mids = u.sband2 + u.sband3;
    let highs = u.sband4 + u.sband5;

    uv *= u.zoom;
    let ca = t * u.rotation_speed * 0.1;
    uv = vec2<f32>(uv.x*cos(ca)-uv.y*sin(ca), uv.x*sin(ca)+uv.y*cos(ca));

    let freq = 2.0 + u.intensity;
    let bolt1 = lightning(uv, t * 2.0, freq);
    let bolt2 = lightning(uv + vec2(3.7, 1.2), t * 1.5 + 100.0, freq * 1.3);
    let bolt3 = lightning(uv - vec2(1.5, 2.8), t * 2.5 + 200.0, freq * 0.8);

    // glow from bolt ridges
    let glow1 = pow(bolt1, 3.0 + bass * u.bass_reactivity);
    let glow2 = pow(bolt2, 3.0 + mids * 2.0);
    let glow3 = pow(bolt3, 4.0 + highs * 3.0);

    let hue = u.color_shift;
    let c_main = vec3<f32>(0.3+0.5*sin(hue*6.28), 0.5+0.4*sin(hue*6.28+1.5), 1.0);
    let c_hot = vec3<f32>(1.0, 0.9, 1.0); // white core
    let c_accent = vec3<f32>(0.8+0.2*sin(hue*6.28+3.0), 0.2, 0.6+0.3*sin(hue*6.28+5.0));

    var col = c_main * glow1 * 0.5;
    col += c_accent * glow2 * 0.3;
    col += c_main.zyx * glow3 * 0.2;

    // white core where bolts are strongest
    let core = max(max(glow1, glow2), glow3);
    col += c_hot * pow(core, 2.0) * 0.3 * u.intensity;

    let bg = vec3<f32>(0.02, 0.01, 0.04);
    col = max(col, bg);

    col += c_hot * u.onset * u.flash_intensity;

    col *= 0.5 + u.energy * 0.5;

    col *= u.brightness;
    col = clamp(col, vec3<f32>(0.005), vec3<f32>(1.0));
    return vec4<f32>(col, 1.0);
}
