#version 410

const vec2 verts[3] = vec2[3](
  vec2(0.0f, 0.5f),
  vec2(-0.5f, -0.5f),
  vec2(0.5f, -0.5f)
);

uniform vec2 screenPos;
uniform float zoomX;
uniform float zoomY;

out vec2 vertex;

void main() {
  vertex = verts[gl_VertexID];
  vec2 pos = vec2(
    (screenPos.x - vertex.x) * zoomX,
    (screenPos.y - vertex.y) * zoomY
  );
  gl_Position = vec4(pos, 0.0, 1.0);
}
