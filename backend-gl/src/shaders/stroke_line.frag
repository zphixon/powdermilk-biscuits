#version 430

layout (location=0) in vec4 fragPos;
layout (location=1) in float pressure;
layout (location=2) in vec3 fragStrokeColor;

layout (location=0) out vec4 color;

void main() {
  color = vec4(fragStrokeColor * pressure, 1.0);
}
