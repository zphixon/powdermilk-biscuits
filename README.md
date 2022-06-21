# tablet-thing

![Screenshot of the text "Powdermilk Biscuits" handwritten on a tablet using this program. Each stroke is a different color, and the strokes are rendered using a cubic Bezier interpolator.](eg/pmb.png)

Uses [a fork](https://github.com/zphixon/winit) of [winit](https://github.com/rust-windowing/winit) that adds the pen inverted state. Only works on Windows.

## Features:

- Line <br> ![](eg/line.png)
- Circle <br> ![](eg/circle.png)
- Circle with pressure scaling <br> ![](eg/circle_pressure.png)
- Points <br> ![](eg/points.png)
- Strokes can be erased, undone, or cleared
- The image can be saved

## Todo:

- Better handling of the effects of pen pressure on stroke width
- Antialiasing
- Bezier/spline rendering
- Geometry-based rather than pixel-based rendering
- Any sort of GUI
