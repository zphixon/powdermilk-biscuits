#version 430

layout (location=0) in vec2 strokePos;
layout (location=1) in float pressure;
layout (location=2) in float zoom;
layout (location=3) in float aspectRatio;

layout (location=0) out vec2 fragPos;

void main() {
  gl_Position = vec4(fragPos * zoom, 0.0, 1.0);
}
