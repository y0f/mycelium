// strobe. warning: epileptic content. aggressive beat-synced geometric flash
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

fn pal(t:f32,s:f32)->vec3<f32>{
    let d=vec3<f32>(0.28+0.35*cos(s*6.283),0.38+0.35*cos(s*6.283+2.1),0.56+0.35*cos(s*6.283+4.2));
    return 0.5+0.5*cos(6.283*(t+d));
}

@fragment fn fs_main(in:VertexOutput)->@location(0) vec4<f32>{
    var p=(in.uv-0.5)*2.0; p.x*=u.res_x/u.res_y;
    let t=u.time*u.speed*2.0; // double speed base for aggression
    let bass=u.sband0+u.sband1; let mids=u.sband2+u.sband3; let highs=u.sband4+u.sband5;
    let energy=u.energy;

    p*=u.zoom;
    let ca=cos(t*u.rotation_speed*0.5); let sa=sin(t*u.rotation_speed*0.5);
    p=vec2<f32>(p.x*ca-p.y*sa,p.x*sa+p.y*ca);

    let r=length(p);
    let a=atan2(p.y,p.x);

    // step functions for harsh edges
    let grid=step(0.0, sin(p.x*8.0*u.intensity+t*3.0)*sin(p.y*8.0*u.intensity-t*2.0));
    let rings=step(0.5, fract(r*4.0*u.intensity-t*4.0));
    let sectors=step(0.0, sin(a*6.0+t*5.0*u.rotation_speed));
    let diag=step(0.0, sin((p.x+p.y)*10.0*u.intensity+t*6.0));

    let beat_flash=step(0.7, 1.0-u.beat_phase); // flash near beat start
    let bass_flash=step(0.3, bass*u.bass_reactivity);

    // XOR-like mixing for hard contrast
    let pat1=abs(grid-rings);
    let pat2=abs(sectors-diag);
    let combined=abs(pat1-pat2);

    // hard switching between palette positions
    let color_idx=floor(t*3.0+bass*u.bass_reactivity*2.0);
    let col1=pal(color_idx*0.1, u.color_shift);
    let col2=pal(color_idx*0.1+0.5, u.color_shift+0.5);
    let col3=vec3<f32>(1.0); // white for max flash

    var col=mix(col1,col2,combined)*u.intensity;

    // alternating black/white/color on beat
    let strobe_phase=step(0.0, sin(t*15.0+bass*u.bass_reactivity*10.0));
    col=mix(col, col3*u.intensity, strobe_phase*beat_flash*u.flash_intensity);

    // bass hit: full screen color blast
    col=mix(col, pal(t*0.5,u.color_shift)*2.0, bass_flash*u.flash_intensity*0.5);

    col+=vec3<f32>(1.0,0.9,1.0)*u.onset*u.flash_intensity*2.0;

    // invert sections based on highs
    let inv=step(0.5, highs*u.intensity);
    col=mix(col, vec3<f32>(1.0)-col, inv*sectors);

    col*=u.brightness;
    // no tonemap; harsh clipping is intentional for strobe
    return vec4<f32>(clamp(col,vec3<f32>(0.0),vec3<f32>(1.0)),1.0);
}
