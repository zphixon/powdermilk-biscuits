#version 140

precision mediump float;

in vec2 vertex;
out vec4 color;

void main() {
  color = vec4(vertex, 0.5, 1.0);
}
