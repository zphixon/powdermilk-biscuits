#version 430

layout (location=0) uniform float erasing;
layout (location=1) uniform float penDown;
layout (location=2) uniform mat4 view;

layout (location=0) in vec2 pos;

layout (location=0) out float fragErasing;
layout (location=1) out float fragPenDown;

void main() {
  gl_Position = view * vec4(pos, 0.5, 1.0);
  fragErasing = erasing;
  fragPenDown = penDown;
}
