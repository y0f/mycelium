// kaleidoscope: mirror folds with FBM
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

const TAU: f32 = 6.28318;

fn hash(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p); let f = fract(p); let w = f*f*(3.0-2.0*f);
    return mix(mix(hash(i), hash(i+vec2(1,0)), w.x), mix(hash(i+vec2(0,1)), hash(i+vec2(1,1)), w.x), w.y);
}
fn fbm(p_in: vec2<f32>) -> f32 {
    var p = p_in; var v = 0.0; var a = 0.5;
    for (var i = 0; i < 5; i++) { v += a * noise(p); p *= 2.0; a *= 0.5; }
    return v;
}

fn kfold(uv: vec2<f32>, segs: f32) -> vec2<f32> {
    let a = atan2(uv.y, uv.x); let r = length(uv);
    let sa = TAU / segs;
    var fa = ((a % sa) + sa) % sa;
    if (fa > sa * 0.5) { fa = sa - fa; }
    return vec2<f32>(cos(fa), sin(fa)) * r;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var p = (in.uv - 0.5) * 2.0; p.x *= u.res_x / u.res_y;
    let t = u.time * u.speed * 0.4;
    let bass = u.sband0 + u.sband1;
    let mids = u.sband2 + u.sband3;
    let highs = u.sband4 + u.sband5;

    p *= u.zoom;
    let ga = t * u.rotation_speed * 0.2;
    p = vec2<f32>(p.x*cos(ga)-p.y*sin(ga), p.x*sin(ga)+p.y*cos(ga));

    let segs = floor(4.0 + mids * 3.0) * u.intensity;
    var kp = kfold(p, max(segs, 3.0));

    // second fold
    kp = vec2<f32>(kp.x*cos(t*0.1)-kp.y*sin(t*0.1), kp.x*sin(t*0.1)+kp.y*cos(t*0.1));
    kp = kfold(kp, max(segs * 0.5 + 2.0, 3.0));

    kp += vec2<f32>(t * 0.12, t * 0.08);

    // bass domain warp
    let w = bass * u.bass_reactivity * 0.2;
    kp += vec2<f32>(sin(kp.y * 4.0 + t) * w, cos(kp.x * 4.0 - t) * w);

    let n1 = fbm(kp * 2.5 + t * 0.1);
    let n2 = fbm(kp * 4.0 - t * 0.15 + vec2<f32>(5.2, 1.3));

    let hue = u.color_shift + n1 * 0.4 + n2 * 0.2 + t * 0.02;
    var col = vec3<f32>(
        0.5 + 0.5 * sin(hue * TAU),
        0.5 + 0.5 * sin(hue * TAU + 2.1),
        0.5 + 0.5 * sin(hue * TAU + 4.2)
    );

    // contrast from FBM edges
    let contrast = abs(n1 - n2) * 3.0;
    col *= 0.4 + contrast * u.intensity;

    // accent color in pattern peaks
    let peak = smoothstep(0.5, 0.8, n1);
    col += vec3<f32>(0.8, 0.3, 1.0) * peak * 0.4 * u.intensity;

    // radial gradient, bright center
    let r = length(p);
    col *= 0.5 + exp(-r * 0.8) * 1.0;

    col += vec3<f32>(0.7, 0.4, 1.0) * u.onset * u.flash_intensity * 0.6;

    col *= u.brightness;
    col = pow(clamp(col, vec3<f32>(0.01), vec3<f32>(1.0)), vec3<f32>(0.9));
    return vec4<f32>(col, 1.0);
}
