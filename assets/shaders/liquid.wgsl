// liquid: metallic fluid with perlin domain warping
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
    let i = floor(p); let f = fract(p);
    let w = f*f*f*(f*(f*6.0-15.0)+10.0); // quintic for smoothness
    return mix(mix(hash(i), hash(i+vec2(1,0)), w.x), mix(hash(i+vec2(0,1)), hash(i+vec2(1,1)), w.x), w.y);
}

// triple domain warp for liquid flow
fn liquid_field(p: vec2<f32>, t: f32) -> f32 {
    let q = vec2<f32>(
        noise(p + vec2(0.0, 0.0) + t * 0.1),
        noise(p + vec2(5.2, 1.3) + t * 0.08)
    );
    let r = vec2<f32>(
        noise(p + q * 4.0 + vec2(1.7, 9.2) + t * 0.05),
        noise(p + q * 4.0 + vec2(8.3, 2.8) + t * 0.06)
    );
    return noise(p + r * 4.0);
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = (in.uv - 0.5) * 2.0; uv.x *= u.res_x / u.res_y;
    let t = u.time * u.speed * 0.3;
    let bass = u.sband0 + u.sband1;
    let mids = u.sband2 + u.sband3;

    uv *= u.zoom;
    let ca = t * u.rotation_speed * 0.05;
    uv = vec2<f32>(uv.x*cos(ca)-uv.y*sin(ca), uv.x*sin(ca)+uv.y*cos(ca));

    // audio warps the flow field
    let warp = bass * u.bass_reactivity * 0.1;
    uv += vec2<f32>(sin(uv.y * 3.0 + t) * warp, cos(uv.x * 3.0 - t) * warp);

    let f1 = liquid_field(uv * (1.0 + u.intensity * 0.5), t);
    let f2 = liquid_field(uv * (1.5 + u.intensity * 0.3) + vec2(10.0, 0.0), t * 0.8);

    // gradient creates an oil-on-water effect
    let hue = u.color_shift + f1 * 0.4 + f2 * 0.2 + t * 0.01;
    let metal = vec3<f32>(
        0.5 + 0.5 * sin(hue * 6.28 * 2.0),
        0.5 + 0.5 * sin(hue * 6.28 * 2.0 + 1.5),
        0.5 + 0.5 * sin(hue * 6.28 * 2.0 + 3.0)
    );

    // sharpen transitions for a liquid look
    let edge = abs(f1 - f2) * 4.0;
    var col = metal * (0.3 + edge * 0.5) * u.intensity;

    // specular highlights
    let spec = pow(max(f1 * 2.0 - 1.0, 0.0), 4.0);
    col += vec3<f32>(1.0, 0.9, 0.8) * spec * 0.4;

    // dark valleys between flows
    let valley = smoothstep(0.4, 0.5, f1) * smoothstep(0.4, 0.5, f2);
    col *= 0.3 + valley * 0.7;

    // mids add color richness
    col = mix(col, metal * 1.5, mids * 0.15);

    let r = length(uv);
    col += metal * exp(-r * 1.5) * 0.15 * (1.0 + bass * u.bass_reactivity * 0.2);

    col += vec3<f32>(0.8, 0.6, 1.0) * u.onset * u.flash_intensity * 0.4;

    col *= u.brightness;
    col = pow(clamp(col, vec3<f32>(0.005), vec3<f32>(1.0)), vec3<f32>(0.85));
    return vec4<f32>(col, 1.0);
}
