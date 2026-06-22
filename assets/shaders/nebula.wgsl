// nebula: deep space gas clouds with volumetric layering
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

fn fbm(p_in: vec2<f32>, t: f32) -> f32 {
    var p = p_in; var v = 0.0; var a = 0.5;
    let c = cos(0.5+t*0.01); let s = sin(0.5+t*0.01);
    let rot = mat2x2<f32>(c,-s,s,c);
    for (var i = 0; i < 5; i++) {
        v += a * noise(p);
        p = rot * p * 2.1 + vec2(1.7, 3.2);
        a *= 0.5;
    }
    return v;
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = (in.uv - 0.5) * 2.0; uv.x *= u.res_x / u.res_y;
    let t = u.time * u.speed * 0.2;
    let bass = u.sband0 + u.sband1;
    let mids = u.sband2 + u.sband3;

    uv *= u.zoom;
    let ca = t * u.rotation_speed * 0.05;
    uv = vec2<f32>(uv.x*cos(ca)-uv.y*sin(ca), uv.x*sin(ca)+uv.y*cos(ca));

    // layered nebula clouds
    let warp = bass * u.bass_reactivity * 0.1;
    let n1 = fbm(uv * 1.0 + t * 0.1, t);
    let n2 = fbm(uv * 2.0 + vec2(n1 * warp, t * 0.08), t * 0.7);
    let n3 = fbm(uv * 0.5 + vec2(n2, n1) * 0.3, t * 1.2);

    // deep space palette: purples, blues, warm accents
    let hue = u.color_shift + n3 * 0.3 + t * 0.005;
    let c1 = vec3<f32>(0.15+0.4*sin(hue*6.28+0.5), 0.05+0.15*sin(hue*6.28+2.8), 0.3+0.4*sin(hue*6.28+4.5));
    let c2 = vec3<f32>(0.4+0.3*sin(hue*6.28+1.5), 0.1+0.2*sin(hue*6.28+3.5), 0.6+0.3*sin(hue*6.28+5.0));
    let c3 = vec3<f32>(0.8, 0.4, 0.2); // warm star core accent

    var col = mix(c1, c2, smoothstep(0.3, 0.7, n1)) * u.intensity;

    // star-like bright points
    let stars = pow(max(noise(uv * 50.0 + t * 0.5) - 0.75, 0.0) * 4.0, 2.0);
    col += c3 * stars * 0.3;

    let depth = n1 * n2 * 3.0;
    col += c2 * depth * 0.2 * u.intensity;

    let r = length(uv);
    col += mix(c1, c3, 0.5) * exp(-r * 1.5) * (0.2 + bass * u.bass_reactivity * 0.15);

    // mids brighten edges
    let edges = abs(n1 - n2) * 2.0;
    col += vec3<f32>(0.5, 0.2, 0.7) * edges * mids * 0.15;

    col += vec3<f32>(0.6, 0.3, 0.8) * u.onset * u.flash_intensity * 0.4;

    col *= u.brightness;
    col = pow(clamp(col, vec3<f32>(0.003), vec3<f32>(1.0)), vec3<f32>(0.9));
    return vec4<f32>(col, 1.0);
}
