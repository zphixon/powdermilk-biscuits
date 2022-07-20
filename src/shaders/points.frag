#version 430

layout (location=0) in vec2 fragPos;
layout (location=1) in float pressure;
layout (location=2) in float drawOrigin;

layout (location=0) out vec4 color;

void main() {
  if (drawOrigin > 0.5) {
    color = vec4(0.0, 0.0, 0.0, 1.0);
  } else {
    color = vec4(fragPos, pressure, 1.0);
  }
}
