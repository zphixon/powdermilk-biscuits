struct Instance {
  @location(0) top_left: vec2<f32>,
  @location(1) tex_coords: vec2<f32>,
  @location(2) width: f32,
  @location(3) height: f32,
};

struct Frag {
  @builtin(position) pos: vec4<f32>,
  @location(0) tex_coords: vec2<f32>,
};

@group(0) @binding(0) var<uniform> view: mat4x4<f32>;
@group(0) @binding(1) var<uniform> atlas_size: vec2<f32>;
@group(1) @binding(0) var atlas_texture: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

@vertex fn vmain(@builtin(vertex_index) index: u32, in: Instance) -> Frag {
  // https://github.com/gfx-rs/naga/issues/920
  // TODO just put this in a Buffer CPU-side
  var tex_coords: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2( in.tex_coords.x             / atlas_size.x,  in.tex_coords.y              / atlas_size.y),
    vec2((in.tex_coords.x + in.width) / atlas_size.x,  in.tex_coords.y              / atlas_size.y),
    vec2( in.tex_coords.x             / atlas_size.x, (in.tex_coords.y + in.height) / atlas_size.y),
    vec2((in.tex_coords.x + in.width) / atlas_size.x, (in.tex_coords.y + in.height) / atlas_size.y),
  );

  var out: Frag;
  out.pos = view * vec4(in.top_left, 0.0, 1.0);
  out.tex_coords = tex_coords[index];
  return out;
}

@fragment fn fmain(in: Frag) -> @location(0) vec4<f32> {
  return textureSample(atlas_texture, atlas_sampler, in.tex_coords);
}

