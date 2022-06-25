//use pixels::{Pixels, SurfaceTexture},
use tablet_thing::{
    graphics::{
        self,
        coords::{ScreenPos, StrokePos},
    },
    input::InputHandler,
    State, StrokeStyle,
};
use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo,
    },
    instance::{Instance, InstanceCreateInfo},
    swapchain::Swapchain,
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

//fn new_pixels(window: &Window) -> Pixels {
//    let size = window.inner_size();
//    let tex = SurfaceTexture::new(size.width, size.height, &window);
//    Pixels::new(size.width, size.height, tex).unwrap()
//}

#[allow(unreachable_code)]
fn main() {
    let required_extensions = vulkano_win::required_extensions();
    let instance = Instance::new(InstanceCreateInfo {
        enabled_extensions: required_extensions,
        ..Default::default()
    })
    .unwrap();

    let ev = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&ev, instance.clone())
        .unwrap();

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (phy_device, queue_family) = PhysicalDevice::enumerate(&instance)
        .filter(|&d| d.supported_extensions().is_superset_of(&device_extensions))
        .filter_map(|d| {
            d.queue_families()
                .find(|&q| q.supports_graphics() && q.supports_surface(&surface).unwrap_or(false))
                .map(|q| (d, q))
        })
        .min_by_key(|(d, _)| match d.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
        })
        .expect("no device found :(");

    println!(
        "device {} ({:?})",
        phy_device.properties().device_name,
        phy_device.properties().device_type,
    );

    let (device, mut queues) = Device::new(
        phy_device,
        DeviceCreateInfo {
            enabled_extensions: phy_device.required_extensions().union(&device_extensions),
            queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
            ..Default::default()
        },
    )
    .unwrap();
    let queue = queues.next().unwrap();

    let (mut swapchain, images) = {
        let surface_caps = phy_device
            .surface_capabilities(&surface, Default::default())
            .unwrap();

        let image_format = Some(
            phy_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        Swapchain::new(
            device.clone(),
            surface.clone(),
            vulkano::swapchain::SwapchainCreateInfo {
                min_image_count: surface_caps.min_image_count,
                image_format,
                image_extent: surface.window().inner_size().into(),
                image_usage: vulkano::image::ImageUsage::color_attachment(),
                composite_alpha: surface_caps
                    .supported_composite_alpha
                    .iter()
                    .next()
                    .unwrap(),
                ..Default::default()
            },
        )
        .unwrap()
    };

    //let mut pixels = new_pixels(&window);

    let mut cursor_visible = true;
    let mut input_handler = InputHandler::default();

    let mut state = State::default();
    println!("stroke style {:?}", state.stroke_style);

    let mut screen_in_paper = StrokePos { x: -2.0, y: 5.33 };
    let mut zoom = 150.;

    ev.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput(KeyboardInput {
                        virtual_keycode: Some(key),
                        state: key_state,
                        ..
                    }),
                ..
            } => {
                use VirtualKeyCode::*;
                input_handler.handle_key(key, key_state);

                if input_handler.just_pressed(Escape) {
                    *control_flow = ControlFlow::Exit;
                }

                if input_handler.just_pressed(C) {
                    state.clear_strokes();
                    surface.window().request_redraw();
                }

                if input_handler.just_pressed(D) {
                    println!("zoom={zoom:.02}");
                    println!("screen_in_paper={screen_in_paper:?}");
                }

                if input_handler.just_pressed(F) {
                    state.fill_brush_head = !state.fill_brush_head;
                    surface.window().request_redraw();
                }

                if input_handler.control() && input_handler.just_pressed(Z) {
                    state.undo_stroke();
                    surface.window().request_redraw();
                }

                if input_handler.just_pressed(Key1)
                    || input_handler.just_pressed(Key2)
                    || input_handler.just_pressed(Key3)
                    || input_handler.just_pressed(Key4)
                    || input_handler.just_pressed(Key5)
                    || input_handler.just_pressed(Key6)
                    || input_handler.just_pressed(Key7)
                    || input_handler.just_pressed(Key8)
                    || input_handler.just_pressed(Key9)
                    || input_handler.just_pressed(Key0)
                {
                    state.stroke_style = unsafe {
                        std::mem::transmute(
                            match key {
                                Key1 => 0,
                                Key2 => 1,
                                Key3 => 2,
                                Key4 => 3,
                                Key5 => 4,
                                Key6 => 5,
                                Key7 => 6,
                                Key8 => 7,
                                Key9 => 8,
                                Key0 => 9,
                                _ => unreachable!(),
                            } % StrokeStyle::NUM_VARIANTS,
                        )
                    };
                    surface.window().request_redraw();

                    println!("stroke style {:?}", state.stroke_style);
                }

                if input_handler.just_pressed(R) {
                    state.use_individual_style = !state.use_individual_style;
                    surface.window().request_redraw();
                }

                if input_handler.just_pressed(S) {
                    let num_string = std::fs::read_to_string("img/num.txt").expect("read num.txt");
                    let num = num_string.trim().parse::<usize>().expect("parse num.txt");
                    let filename = format!("img/strokes{num}.png");

                    // when we render with a real graphics library, we'll compute the geometry of
                    // each stroke and just render it like a normal person. when we want the full
                    // overview like what we're trying to do here we'll render into an image
                    // target, mapping each sample so that the far bounds of the stroke space
                    // correspond to 1/-1.

                    if input_handler.shift() {
                        let mut min_x = f64::INFINITY;
                        let mut max_x = -f64::INFINITY;
                        let mut min_y = f64::INFINITY;
                        let mut max_y = -f64::INFINITY;
                        let mut max_rad = -f64::INFINITY;
                        for stroke in state.strokes.iter() {
                            if stroke.style == StrokeStyle::Circles && stroke.brush_size > max_rad {
                                max_rad = stroke.brush_size;
                            }

                            for point in stroke.points.iter() {
                                if point.pos.x > max_x {
                                    max_x = point.pos.x;
                                }
                                if point.pos.x < min_x {
                                    min_x = point.pos.x;
                                }
                                if point.pos.y > max_y {
                                    max_y = point.pos.y;
                                }
                                if point.pos.y < min_y {
                                    min_y = point.pos.y;
                                }
                            }
                        }

                        let margin = 20. + max_rad;

                        let top_left_stroke = StrokePos { x: min_x, y: max_y };
                        let bottom_right_stroke = StrokePos { x: max_x, y: min_y };
                        let bottom_right_screen =
                            ScreenPos::from_stroke(bottom_right_stroke, 150., top_left_stroke);
                        let width = bottom_right_screen.x + 2 * margin as isize;
                        let height = bottom_right_screen.y + 2 * margin as isize;
                        let diff = bottom_right_stroke - top_left_stroke;
                        let zoom_overview = width as f64 / diff.x;
                        let width = width.try_into().unwrap();
                        let height = height.try_into().unwrap();

                        let image = image::RgbaImage::new(width, height);

                        let mut container = image.into_raw();

                        graphics::clear(container.as_mut_slice());
                        state.draw_strokes(
                            container.as_mut_slice(),
                            width as usize,
                            height as usize,
                            zoom_overview,
                            top_left_stroke,
                        );

                        let image = image::RgbaImage::from_raw(width, height, container)
                            .expect("image from raw");
                        image.save(&filename).expect(&format!("save {filename}"));
                    } else {
                        let PhysicalSize { width, height } = surface.window().inner_size();
                        //let frame = pixels.get_frame();
                        //graphics::clear(frame);
                        //state.draw_strokes(
                        //    frame,
                        //    width as usize,
                        //    height as usize,
                        //    zoom,
                        //    screen_in_paper,
                        //);

                        //image::save_buffer(
                        //    &filename,
                        //    frame,
                        //    width,
                        //    height,
                        //    image::ColorType::Rgba8,
                        //)
                        //.expect(&format!("save {filename}"));
                    }

                    let next_num = num + 1;
                    std::fs::write("img/num.txt", format!("{next_num}")).expect("write num.txt");
                    println!("wrote image as {filename}");

                    surface.window().request_redraw();
                }
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::Touch(touch),
                ..
            } => {
                cursor_visible = false;
                surface.window().set_cursor_visible(cursor_visible);
                state.update(touch, zoom, screen_in_paper);
                surface.window().request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                let zoom_in = match delta {
                    MouseScrollDelta::LineDelta(_, y) if y.is_sign_positive() => true,
                    MouseScrollDelta::PixelDelta(pos) if pos.y.is_sign_positive() => true,
                    MouseScrollDelta::LineDelta(_, y) if y.is_sign_negative() => false,
                    MouseScrollDelta::PixelDelta(pos) if pos.y.is_sign_negative() => false,
                    _ => unreachable!(),
                };
                const ZOOM_SPEED: f64 = 3.;

                let PhysicalSize { width, height } = surface.window().inner_size();
                let dzoom = if zoom_in { ZOOM_SPEED } else { -ZOOM_SPEED };
                let dscreen_in_paper = if zoom_in {
                    let x = (width as f64 / 2.) / zoom;
                    let y = -(height as f64 / 2.) / zoom;
                    StrokePos { x, y }
                } else {
                    let x = -(width as f64 / 2.) / zoom;
                    let y = (height as f64 / 2.) / zoom;
                    StrokePos { x, y }
                };

                zoom += dzoom;
                let next_sip = screen_in_paper + (dscreen_in_paper * (1. / zoom));
                if next_sip.x.is_finite() && next_sip.y.is_finite() {
                    screen_in_paper = next_sip;
                }

                surface.window().request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter(c @ ('[' | ']')),
                ..
            } => {
                match c {
                    '[' => state.decrease_brush(),
                    ']' => state.increase_brush(),
                    _ => unreachable!(),
                };
                surface.window().request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                input_handler.handle_mouse_button(button, state);
            }

            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                let prev = input_handler.cursor_pos();
                input_handler.handle_mouse_move(position);

                if input_handler.button_down(MouseButton::Left) {
                    let next = input_handler.cursor_pos();
                    let diff = StrokePos::from_screen_pos(prev, zoom, screen_in_paper)
                        - StrokePos::from_screen_pos(next, zoom, screen_in_paper);
                    screen_in_paper = screen_in_paper + diff;
                    surface.window().request_redraw();
                }

                if !cursor_visible {
                    cursor_visible = true;
                    surface.window().set_cursor_visible(cursor_visible);
                    surface.window().request_redraw();
                }
            }

            Event::RedrawRequested(_) => {
                //let frame = pixels.get_frame();
                //graphics::clear(frame);

                //let PhysicalSize { width, height } = window.inner_size();
                //let (width, height) = (width as usize, height as usize);

                //state.draw_strokes(frame, width, height, zoom, screen_in_paper);

                //if !cursor_visible {
                //    graphics::put_circle_absolute(
                //        frame,
                //        width,
                //        height,
                //        ScreenPos::from_stroke(state.stylus.pos, zoom, screen_in_paper),
                //        match (state.stylus.inverted(), state.stylus.down()) {
                //            (true, true) => [0xfa, 0x34, 0x33],
                //            (true, false) => [0x53, 0x11, 0x11],
                //            (false, true) => [0xff, 0xff, 0xff],
                //            (false, false) => [0x55, 0x55, 0x55],
                //        },
                //        state.brush_size,
                //    );
                //}

                //pixels.render().unwrap();
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                //pixels = new_pixels(&surface.window());
                surface.window().request_redraw();
            }

            _ => {}
        }
    });
}
