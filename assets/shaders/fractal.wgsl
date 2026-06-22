// fractal flame / kali hybrid with orbit trap coloring.
// designed so no single parameter can kill the image.
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

fn rot(a: f32) -> mat2x2<f32> {
    let c = cos(a); let s = sin(a);
    return mat2x2<f32>(c, s, -s, c);
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = u.time * max(u.speed, 0.05);
    let bass = u.sband0 + u.sband1;
    let mids = u.sband2 + u.sband3;
    let highs = u.sband4 + u.sband5;

    var p = (in.uv - 0.5) * 2.0;
    p.x *= u.res_x / u.res_y;

    // floor at 0.3 so fractal never disappears
    p *= max(u.zoom, 0.3);

    p = p * rot(t * u.rotation_speed * 0.15);

    // offset drifts slowly so the fractal keeps evolving
    p += vec2<f32>(sin(t * 0.07) * 0.5, cos(t * 0.11) * 0.5);

    // kali fold + inversion + rotation per step; detail everywhere, not just at boundaries
    var z = p;
    var color_acc = vec3<f32>(0.0);
    var trap1 = 100.0;
    var trap2 = 100.0;
    var trap3 = 100.0;

    // seed morphs with time and audio
    let seed_x = 0.7 + sin(t * 0.13) * 0.2 + bass * u.bass_reactivity * 0.02;
    let seed_y = 0.7 + cos(t * 0.09) * 0.2 + mids * u.bass_reactivity * 0.015;

    for (var i = 0; i < 24; i++) {
        z = abs(z);

        // inversion z = z / |z|^2, core of kali fractals
        let r2 = dot(z, z);
        z /= max(r2, 0.01);

        // subtract seed creates the fractal structure
        z -= vec2<f32>(seed_x, seed_y);

        // rotate between iterations prevents symmetry collapse
        z = z * rot(0.6 + f32(i) * 0.05 + highs * 0.02);

        // orbit traps: proximity to geometric shapes
        trap1 = min(trap1, abs(z.x));           // vertical line trap
        trap2 = min(trap2, abs(z.y));           // horizontal line trap
        trap3 = min(trap3, length(z) - 0.5);   // circle trap
    }

    // three orbit traps blended into RGB channels
    let t1 = exp(-trap1 * 10.0);
    let t2 = exp(-trap2 * 10.0);
    let t3 = exp(-abs(trap3) * 7.0);

    // base color from traps, rotated by color_shift
    let cs = u.color_shift * 6.283;
    var col = vec3<f32>(
        t1 * (0.6 + 0.4 * sin(cs)) + t3 * (0.3 + 0.3 * cos(cs + 2.0)),
        t2 * (0.6 + 0.4 * sin(cs + 2.1)) + t1 * (0.2 + 0.2 * cos(cs + 4.0)),
        t3 * (0.6 + 0.4 * sin(cs + 4.2)) + t2 * (0.2 + 0.2 * cos(cs))
    );

    // intensity controls saturation and contrast, not brightness.
    // at 0 muted, at 5 vivid neon.
    let int = max(u.intensity, 0.0) + 0.3;
    let luma = dot(col, vec3<f32>(0.3, 0.6, 0.1));
    col = mix(vec3<f32>(luma * 0.4), col, clamp(int, 0.3, 3.0));
    col *= 0.1 + int * 0.06;

    // bass deepens saturation, mids shift hue
    col = mix(col, col * col * 2.5, clamp(bass * u.bass_reactivity * 0.05, 0.0, 0.4));

    // brief glow on beat, never washes out
    let beat = pow(max(1.0 - u.beat_phase, 0.0), 6.0);
    col *= 1.0 + beat * 0.15;

    col += vec3<f32>(0.35, 0.15, 0.55) * u.onset * u.flash_intensity * 0.35;

    col *= max(u.brightness, 0.1);

    // soft clamp preserves hue, never blows to white
    let pk = max(max(col.x, col.y), col.z);
    if pk > 1.0 { col /= pk; }

    return vec4<f32>(max(col, vec3<f32>(0.0)), 1.0);
}
