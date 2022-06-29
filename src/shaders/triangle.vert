#version 140

const vec2 verts[3] = vec2[3](
  vec2(0.0f, 0.5f),
  vec2(-0.5f, -0.5f),
  vec2(0.5f, -0.5f)
);

out vec2 vertex;

void main() {
  vertex = verts[gl_VertexID];
  gl_Position = vec4(vertex, 0.0, 1.0);
}
