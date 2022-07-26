struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
};

@vertex fn vmain(
  @builtin(vertex_index) index: u32,
) -> VertexOutput {
  var out: VertexOutput;
  let x = f32(1 - i32(index)) * 0.5;
  let y = f32(i32(index & 1u) * 2 - 1) * 0.5;
  out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
  return out;
}

@fragment fn fmain() -> @location(0) vec4<f32> {
  return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
