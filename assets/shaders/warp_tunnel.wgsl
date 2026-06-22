// warp tunnel: infinite tunnel with domain warping and layered color
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

fn hash(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p); let f = fract(p); let w = f*f*(3.0-2.0*f);
    return mix(mix(hash(i), hash(i+vec2(1,0)), w.x), mix(hash(i+vec2(0,1)), hash(i+vec2(1,1)), w.x), w.y);
}
fn fbm3(p_in: vec2<f32>) -> f32 {
    var p = p_in; var v = 0.0; var a = 0.5;
    for (var i = 0; i < 3; i++) { v += a * noise(p); p *= 2.2; a *= 0.5; }
    return v;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = (in.uv - 0.5) * 2.0;
    uv.x *= u.res_x / u.res_y;
    let t = u.time * u.speed;
    let bass = u.sband0 + u.sband1;
    let mids = u.sband2 + u.sband3;
    let highs = u.sband4 + u.sband5;

    uv *= u.zoom;
    let ca = t * u.rotation_speed * 0.08;
    uv = vec2<f32>(uv.x*cos(ca)-uv.y*sin(ca), uv.x*sin(ca)+uv.y*cos(ca));

    // polar coords for tunnel
    let r = length(uv);
    let a = atan2(uv.y, uv.x) / 3.14159;
    let depth = 0.5 / (r + 0.01);

    // moving through tunnel
    let forward = depth + t * 0.8;
    let tunnel_uv = vec2<f32>(a, forward);

    // intensity scales pattern complexity
    let warp = bass * u.bass_reactivity * 0.3;
    let freq = 2.0 + u.intensity * 3.0;
    let n1 = fbm3(tunnel_uv * freq + vec2<f32>(t * 0.2, 0.0));
    let n2 = fbm3(tunnel_uv * freq * 1.7 + vec2<f32>(n1 * warp, t * 0.15));
    let n3 = fbm3(tunnel_uv * 8.0 - vec2<f32>(t * 0.1, n2 * warp));

    // color bands rotating with audio
    let hue1 = u.color_shift + depth * 0.05 + n1 * 0.3;
    let hue2 = u.color_shift + 0.33 + a * 0.2 + n2 * 0.2;
    let hue3 = u.color_shift + 0.66 + n3 * 0.4;

    let c1 = vec3<f32>(0.5+0.5*sin(hue1*6.28+0.0), 0.5+0.5*sin(hue1*6.28+2.1), 0.5+0.5*sin(hue1*6.28+4.2));
    let c2 = vec3<f32>(0.5+0.5*sin(hue2*6.28+1.0), 0.5+0.5*sin(hue2*6.28+3.1), 0.5+0.5*sin(hue2*6.28+5.2));
    let c3 = vec3<f32>(0.5+0.5*sin(hue3*6.28+2.0), 0.5+0.5*sin(hue3*6.28+0.5), 0.5+0.5*sin(hue3*6.28+3.8));

    var col = c1 * n1 * 0.5 + c2 * n2 * 0.4 + c3 * n3 * 0.3;

    // depth fog: brighter in center, dark at edges
    let depth_fade = exp(-r * 1.5);
    col *= depth_fade * 2.0 + 0.15;

    // ring structures at depth intervals
    let rings = 0.5 + 0.5 * sin(depth * 20.0 - t * 3.0 + bass * u.bass_reactivity * 5.0);
    col *= 0.7 + rings * 0.5 * u.intensity;

    col += c1 * exp(-r * 4.0) * (0.5 + bass * u.bass_reactivity * 0.3);

    col += vec3<f32>(0.8, 0.4, 1.0) * u.onset * u.flash_intensity * depth_fade;

    col *= u.brightness * u.intensity;
    col = pow(clamp(col, vec3<f32>(0.005), vec3<f32>(1.0)), vec3<f32>(0.85)); // gamma boost
    return vec4<f32>(col, 1.0);
}
