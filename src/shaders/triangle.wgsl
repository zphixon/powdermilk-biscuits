struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
};

@vertex fn main(
  @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
  var out: VertexOutput;
  let x = f32(1 - i32(vertex_index)) * 0.5;
  let y = f32(i32(vertex_index & 1u) * 2 - 1) * 0.5;
  out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
  return out;
}
