# Powdermilk Biscuits

![Screenshot of the text "Powdermilk Biscuits" handwritten on a tablet using this program. Each stroke is a different color, and the strokes are rendered using a cubic Bezier interpolator.](pmb.png)

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
- Finger gestures
- Stroke rendering revamp
  - Infinite scroll, chunks
  - Pressure-based line weight
  - Caps and joins
  - Correct handling of color space (wgpu impl)

## Notes

Coordinate types:
- `PhysicalPosition` - pixels in window, provided by winit
- `GlPos` - NDC used to calculate where the user is clicking
- `StrokePoint` - stroke position relative to NDC origin in stroke space
- `StrokePos` - stroke position relative to stroke space origin

Finger/stylus interaction:
- `Touch` events have a unique ID to represent different touches. The behavior for recognizing touch gestures should be:
  - When we get a `Touch` event, remember the ID. Set the pen state to down.
  - While the pen is down, if we receive another `Touch` event with a different ID:
    - Remember the ID of the new touch
    - Remove the stroke currently being drawn, set the pen state to up
    - Handle the appropriate gesture based on how many concurrent touches there are
    - Wait until all the touches are ended
- Settings - library function that loads and validates settings file, used by build script to check per-platform builds
  - Ignore touch inputs entirely
  - Use touch inputs only for gestures
  - Use mouse inputs as finger/stylus inputs
  - Gesture for each number of fingers
  - Tap gestures

![Gesture state diagram](gesture-state.png)

Optimizations:
- For both backends, we're checking every stroke every frame whether it's been buffered or not
- We probably don't need to store the spline CPU-side, we could just use a ring buffer and keep track of what mesh data hasn't been buffered to the GPU yet
- Save rendered framebuffers and sample them when possible so we don't need to loop through all the strokes every frame

Keybinds:
- c: clear strokes
- d: print strokes
- a: toggle antialiasing
- p: change GL primitives
- ctrl+z: undo stroke
- z: reset origin and zoom
- e: invert stylus state
- ctrl+o: read file
- ctrl+s: save file
- shift+s: save as image

[Polar stroking tesselation](https://dl.acm.org/doi/pdf/10.1145/3386569.3392458)

![Plot of a Catmull-Rom spline with ribs](rib-plot.png)
