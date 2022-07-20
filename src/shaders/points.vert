#version 430

layout (location=0) in vec2 strokePos;
layout (location=1) in float pressure;

layout (location=0) uniform mat4 view;
layout (location=1) uniform mat4 proj;

layout (location=0) out vec4 fragPos;
layout (location=1) out float fragPressure;

void main() {
  vec4 pos = proj * view * vec4(strokePos, 0.0, 1.0);
  pos.z = 0;
  gl_Position = pos;

  fragPos = pos;
  fragPressure = pressure;
}
