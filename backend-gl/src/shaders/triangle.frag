#version 430

precision mediump float;

layout (location=0) in vec2 vertex;
layout (location=0) out vec4 color;

void main() {
  color = vec4(vertex, 0.5, 1.0);
}
