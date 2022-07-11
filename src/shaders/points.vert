#version 430

layout (location=0) in vec2 strokePos;
layout (location=1) in float pressure;

layout (location=0) uniform vec2 screenPos;
layout (location=1) uniform float zoomX;
layout (location=2) uniform float zoomY;

layout (location=0) out vec2 fragPos;
layout (location=1) out float fragPressure;

void main() {
  vec2 diff = strokePos - screenPos;
  vec2 pos = vec2(
    zoomX * diff.x,
    zoomY * -diff.y
  );
  fragPos = pos;
  fragPressure = pressure;
  gl_Position = vec4(pos, 0.0, 1.0);
}
