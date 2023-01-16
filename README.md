# Powdermilk Biscuits

![Screenshot of the text "Powdermilk Biscuits" handwritten on a tablet using this program. The pen strokes are rendered with stroke weight corresponding to the pressure of the pen.](pmb.png)

Heavens, it's tasty.

## Building:

The build has visited the local mage and is no longer cursed. Just `git clone` and `cargo b --release`! **MSRV 1.65.0**

## Features:

- Strokes can be drawn, undone, and erased
- Files can be saved and opened

## Todo:

- Gui
  - Investigate [fluent](https://projectfluent.org/)
    - The current solution is naive A.F.
  - Bookmark system
    - Click a button and it zooms you to the bookmark's location
  - Color palette
    - Customizable per-file
  - All kinds of UI customization
  - Better keyboard combinations
    - Configuring them in the settings menu
  - Text input?
- Good finger gestures
  - Correct handling of multitouch
- Stroke rendering revamp
  - Infinite zoom, chunks
  - Correct handling of color space
- Fully commit to either WGPU or OpenGL for rendering
- Better config system
  - More sophisticated device configuration
- Better architecture
  - Tessellation/render/dialog boxes on another thread
