struct Vert {
  @location(0) pos: vec2<f32>,
  @location(1) pressure: f32,
};

struct Frag {
  @builtin(position) pos: vec4<f32>,
  @location(0) pressure: f32,
};

@vertex fn vmain(in: Vert) -> Frag {
  var out: Frag;

  out.pos = vec4<f32>(in.pos, 0.0, 1.0);
  out.pressure = in.pressure;

  return out;
}

@fragment fn fmain(in: Frag) -> @location(0) vec4<f32> {
  return vec4<f32>(0.5, in.pressure, 0.5, 1.0);
}
