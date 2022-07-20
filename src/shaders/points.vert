#version 430

layout (location=0) in vec2 strokePos;
layout (location=1) in float pressure;

layout (location=0) uniform vec2 sip;
layout (location=1) uniform float zoom;
layout (location=2) uniform float width;
layout (location=3) uniform float height;

layout (location=0) out vec2 fragPos;
layout (location=1) out float fragPressure;
layout (location=2) out float fragDrawOrigin;

void main() {
  vec2 diff = strokePos - sip;
  vec2 pos = vec2(
    zoom * diff.x * height / width,
    zoom * diff.y
  );
  gl_Position = vec4(pos, 0.0, 1.0);
  gl_PointSize = pressure * 20;

  fragPos = pos;
  fragPressure = pressure;
}
