#version 430

const vec2 verts[3] = vec2[3](
  vec2(0.0f, 0.5f),
  vec2(-0.5f, -0.5f),
  vec2(0.5f, -0.5f)
);

layout (location=0) uniform vec2 screenPos;
layout (location=1) uniform float zoomX;
layout (location=2) uniform float zoomY;

layout (location=0) out vec2 vertex;

void main() {
  vertex = verts[gl_VertexID];
  vec2 pos = vec2(
    (screenPos.x - vertex.x) * zoomX,
    (screenPos.y - vertex.y) * zoomY
  );
  gl_Position = vec4(pos, 0.0, 1.0);
}
