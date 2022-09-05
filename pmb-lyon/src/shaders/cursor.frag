#version 430

layout (location=0) in float erasing;
layout (location=1) in float penDown;

layout (location=0) out vec4 color;

const vec4 eraserDownColor = vec4(0.980, 0.203, 0.200, 1.0);
const vec4 eraserUpColor   = vec4(0.325, 0.067, 0.067, 1.0);
const vec4 penDownColor    = vec4(1.000, 1.000, 1.000, 1.0);
const vec4 penUpColor      = vec4(0.333, 0.333, 0.333, 1.0);

void main() {
  color = mix(
    mix(penUpColor, penDownColor, penDown),
    mix(eraserUpColor, eraserDownColor, penDown),
    erasing
  );
}
