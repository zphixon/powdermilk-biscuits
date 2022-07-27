let eraserDownColor: vec4<f32> = vec4<f32>(0.980, 0.203, 0.200, 1.0);
let eraserUpColor: vec4<f32>   = vec4<f32>(0.325, 0.067, 0.067, 1.0);
let penDownColor: vec4<f32>    = vec4<f32>(1.000, 1.000, 1.000, 1.0);
let penUpColor: vec4<f32>      = vec4<f32>(0.333, 0.333, 0.333, 1.0);

struct Frag {
  @builtin(position) pos: vec4<f32>,
  @location(0) penDown: f32,
  @location(1) erasing: f32,
}

@group(0) @binding(0) var<uniform> view: mat4x4<f32>;

var<push_constant> penState: vec2<f32>;

@vertex fn vmain(@location(0) in: vec2<f32>) -> Frag {
  var out: Frag;
  out.pos = view * vec4<f32>(in, 0.5, 1.0);
  out.penDown = penState.x;
  out.erasing = penState.y;
  return out;
}

@fragment fn fmain(in: Frag) -> @location(0) vec4<f32> {
  return mix(
    mix(penUpColor, penDownColor, in.penDown),
    mix(eraserUpColor, eraserDownColor, in.penDown),
    in.erasing,
  );
}
