struct Vert {
  @location(0) pos: vec2<f32>,
  @location(1) pressure: f32,
};

struct Frag {
  @builtin(position) pos: vec4<f32>,
  @location(0) pressure: f32,
  @location(1) color: vec3<f32>,
};

@group(0) @binding(0) var<uniform> view: mat4x4<f32>;
@group(1) @binding(0) var<uniform> color: vec3<f32>;

@vertex fn vmain(in: Vert) -> Frag {
  var out: Frag;

  out.pos = view * vec4<f32>(in.pos, 0.0, 1.0);
  out.pressure = in.pressure;
  out.color = color;

  return out;
}

@fragment fn fmain(in: Frag) -> @location(0) vec4<f32> {
  return vec4<f32>(in.color, in.pressure);
}
