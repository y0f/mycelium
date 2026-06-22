// sacred geometry: raymarched SDF with iridescent lighting
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

fn ry(a:f32)->mat3x3<f32>{let c=cos(a);let s=sin(a);return mat3x3<f32>(c,0,s,0,1,0,-s,0,c);}
fn rx(a:f32)->mat3x3<f32>{let c=cos(a);let s=sin(a);return mat3x3<f32>(1,0,0,0,c,-s,0,s,c);}

fn smin(a:f32,b:f32,k:f32)->f32{let h=clamp(0.5+0.5*(b-a)/k,0.0,1.0);return mix(b,a,h)-k*h*(1.0-h);}

fn scene(p:vec3<f32>, t:f32, bass:f32) -> f32 {
    var rp = ry(t * 0.4) * p;
    rp = rx(t * 0.25) * rp;

    // torus
    let q = vec2<f32>(length(rp.xz) - 1.0, rp.y);
    let torus = length(q) - (0.3 + bass * 0.1);

    // octahedron
    let ap = abs(rp);
    let octa = (ap.x + ap.y + ap.z - 1.0) * 0.577;

    // gyroid
    let gyroid = abs(dot(sin(rp * 4.0), cos(rp.zxy * 4.0))) * 0.25 - 0.08;

    return smin(smin(torus, octa, 0.3 + bass * 0.2), gyroid, 0.2);
}

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = (in.uv - 0.5) * 2.0; uv.x *= u.res_x / u.res_y;
    let t = u.time * u.speed * 0.4;
    let bass = u.sband0 + u.sband1;
    let mids = u.sband2 + u.sband3;

    // camera orbit
    let dist = 2.0 + 6.0 / max(u.zoom, 0.1); // larger zoom = closer
    let ang = t * u.rotation_speed * 0.25;
    let cp = vec3<f32>(sin(ang)*dist, sin(t*u.rotation_speed*0.1)*1.0, cos(ang)*dist);
    let fw = normalize(-cp);
    let rt = normalize(cross(fw, vec3(0,1,0)));
    let up = cross(rt, fw);
    let rd = normalize(rt*uv.x + up*uv.y + fw*2.0);

    let b = bass * u.bass_reactivity * 0.3;

    // raymarch, 40 steps
    var total = 0.0;
    var hit = false;
    var hp = vec3<f32>(0.0);
    for (var i = 0; i < 40; i++) {
        hp = cp + rd * total;
        let d = scene(hp, t, b);
        if (d < 0.001) { hit = true; break; }
        if (total > 15.0) { break; }
        total += d;
    }

    var col: vec3<f32>;
    if (hit) {
        // normal
        let e = vec2<f32>(0.001, 0.0);
        let n = normalize(vec3<f32>(
            scene(hp+e.xyy,t,b)-scene(hp-e.xyy,t,b),
            scene(hp+e.yxy,t,b)-scene(hp-e.yxy,t,b),
            scene(hp+e.yyx,t,b)-scene(hp-e.yyx,t,b)
        ));

        let light = normalize(vec3<f32>(2.0, 3.0, 1.0));
        let diff = max(dot(n, light), 0.0) * 0.8 + 0.2;
        let spec = pow(max(dot(reflect(rd, n), light), 0.0), 32.0);

        // fresnel + iridescent color
        let fres = pow(1.0 - max(dot(n, -rd), 0.0), 4.0);
        let hue = u.color_shift + fres * 0.5 + dot(hp, vec3(0.1)) + t * 0.03;
        let irid = vec3<f32>(0.5+0.5*sin(hue*6.28), 0.5+0.5*sin(hue*6.28+2.1), 0.5+0.5*sin(hue*6.28+4.2));

        // base color from position
        let base_hue = u.color_shift + length(hp) * 0.15 + t * 0.02;
        let base = vec3<f32>(0.5+0.5*sin(base_hue*6.28+0.5), 0.3+0.5*sin(base_hue*6.28+2.5), 0.7+0.3*sin(base_hue*6.28+4.5));

        col = base * diff * u.intensity + irid * fres * 0.7 + vec3(1.0) * spec * 0.4;

        // AO approximation
        let ao = 1.0 - exp(-total * 0.15);
        col *= ao;

        // audio glow on surface
        col += irid * mids * 0.1;
    } else {
        // background: dark gradient
        let bg_hue = u.color_shift + uv.y * 0.15 + t * 0.01;
        col = vec3<f32>(0.02+0.03*sin(bg_hue*6.28), 0.01+0.02*sin(bg_hue*6.28+2.1), 0.05+0.04*sin(bg_hue*6.28+4.2));
        col += vec3<f32>(0.03, 0.01, 0.06) * exp(-length(uv) * 0.5); // center glow
    }

    col += vec3<f32>(0.5, 0.3, 0.8) * u.onset * u.flash_intensity * 0.4;
    col *= u.brightness;
    col = clamp(col, vec3<f32>(0.003), vec3<f32>(1.0));
    return vec4<f32>(col, 1.0);
}
