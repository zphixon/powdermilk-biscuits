#version 430

layout (location=0) in vec2 fragPos;

layout (location=0) out vec4 color;

void main() {
  color = vec4(fragPos, 0.0, 1.0);
}
