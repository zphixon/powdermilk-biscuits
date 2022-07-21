# tablet-thing

![Screenshot of the text "Powdermilk Biscuits" handwritten on a tablet using this program. Each stroke is a different color, and the strokes are rendered using a cubic Bezier interpolator.](eg/pmb.png)

Uses [a fork](https://github.com/zphixon/winit) of [winit](https://github.com/rust-windowing/winit) that adds the pen inverted state. Only works on Windows.

## Building:

The winit fork is cloned automatically if you cloned with `--recurse-submodules`, but glutin's `glutin/glutin/Cargo.toml` needs to be manually edited to point the winit dependency to the `winit2` directory. Yes, this is cursed. I apologize.

## Features:

- Strokes can be drawn with a number of stroke styles, erased, undone, and cleared
- The image can be saved

## Todo:

- Better handling of the effects of pen pressure on stroke width
- Antialiasing
- Geometry-based rather than pixel-based rendering, includes compositing
- Any sort of GUI
- Graphics using Vulkano or Ash?

## Notes

Coordinate types:
- `PhysicalPosition` - pixels in window, provided by winit
- `GlPos` - NDC used to calculate where the user is clicking
- `StrokePoint` - stroke position relative to NDC origin in stroke space
- `StrokePos` - stroke position relative to stroke space origin

