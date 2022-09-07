# Powdermilk Biscuits

![Screenshot of the text "Powdermilk Biscuits" handwritten on a tablet using this program. Each stroke is a different color, and the strokes are rendered with line segments whose stroke width correspond to the pressure of the pen.](pmb.png)

Heavens, it's tasty.

## Building:

The build has visited the local mage and is no longer cursed. Just `git clone` and `cargo b --release`!

## Features:

- Strokes can be drawn, undone, and erased
- Files can be saved and opened

## Todo:

- Any sort of GUI
  - Undo system
  - Layers?
  - Customization
- Good finger gestures
- Stroke rendering revamp
  - Infinite scroll, chunks
  - Correct handling of color space (wgpu impl)
- Fully commit to either WGPU or OpenGL for rendering
- Config system
  - Maybe more sophisticated device handling
