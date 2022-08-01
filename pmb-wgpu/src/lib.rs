use powdermilk_biscuits::{
    event::{PenInfo, Touch, TouchPhase},
    graphics::{ColorExt, PixelPos, StrokePoint},
    input::{ElementState, Keycode, MouseButton},
    stroke::Stroke,
    State,
};
use std::mem::size_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState, Buffer,
    BufferAddress, BufferBindingType, BufferUsages, Color as WgpuColor, ColorTargetState,
    ColorWrites, CommandEncoderDescriptor, Device, DeviceDescriptor, Face, Features, FragmentState,
    FrontFace, Instance, Limits, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor,
    PolygonMode, PowerPreference, PresentMode, PrimitiveState, PrimitiveTopology,
    PushConstantRange, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, ShaderStages, Surface, SurfaceConfiguration,
    SurfaceError, TextureFormat, TextureUsages, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        ElementState as WinitElementState, MouseButton as WinitMouseButton,
        PenInfo as WinitPenInfo, Touch as WinitTouch, TouchPhase as WinitTouchPhase,
        VirtualKeyCode as WinitKeycode,
    },
    window::Window,
};

pub type WgslState = State<WgpuBackend, StrokeBackend>;

const NUM_SEGMENTS: usize = 50;

#[derive(Debug, Default, Clone, Copy)]
pub struct WgpuBackend;

impl powdermilk_biscuits::Backend for WgpuBackend {
    type Ndc = WgpuNdc;

    fn pixel_to_ndc(&self, width: u32, height: u32, pos: PixelPos) -> Self::Ndc {
        pixel_to_ndc(width, height, pos)
    }

    fn ndc_to_pixel(&self, width: u32, height: u32, pos: Self::Ndc) -> PixelPos {
        ndc_to_pixel(width, height, pos)
    }

    fn ndc_to_stroke(&self, width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint {
        ndc_to_stroke(width, height, zoom, ndc)
    }

    fn stroke_to_ndc(&self, width: u32, height: u32, zoom: f32, point: StrokePoint) -> Self::Ndc {
        stroke_to_ndc(width, height, zoom, point)
    }
}

pub fn view_matrix(
    zoom: f32,
    scale: f32,
    size: PhysicalSize<u32>,
    origin: StrokePoint,
) -> glam::Mat4 {
    let PhysicalSize { width, height } = size;
    let xform = stroke_to_ndc(width, height, zoom, origin);
    glam::Mat4::from_scale_rotation_translation(
        glam::vec3(scale / width as f32, scale / height as f32, 1.0),
        glam::Quat::IDENTITY,
        glam::vec3(xform.x, xform.y, 0.0),
    )
}

#[derive(Debug)]
pub struct StrokeBackend {
    pub points: Buffer,
    pub pressure: Buffer,
    pub dirty: bool,
}

impl powdermilk_biscuits::StrokeBackend for StrokeBackend {
    fn make_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

pub fn physical_pos_to_pixel_pos(pos: PhysicalPosition<f64>) -> PixelPos {
    PixelPos {
        x: pos.x as f32,
        y: pos.y as f32,
    }
}

pub fn glutin_to_pmb_pen_info(pen_info: WinitPenInfo) -> PenInfo {
    PenInfo {
        barrel: pen_info.barrel,
        inverted: pen_info.inverted,
        eraser: pen_info.eraser,
    }
}

pub fn glutin_to_pmb_touch_phase(phase: WinitTouchPhase) -> TouchPhase {
    match phase {
        WinitTouchPhase::Started => TouchPhase::Start,
        WinitTouchPhase::Moved => TouchPhase::Move,
        WinitTouchPhase::Ended => TouchPhase::End,
        WinitTouchPhase::Cancelled => TouchPhase::Cancel,
    }
}

pub fn winit_to_pmb_touch(touch: WinitTouch) -> Touch {
    Touch {
        force: touch.force.map(|f| f.normalized()),
        phase: glutin_to_pmb_touch_phase(touch.phase),
        location: physical_pos_to_pixel_pos(touch.location),
        pen_info: touch.pen_info.map(glutin_to_pmb_pen_info),
    }
}

pub fn winit_to_pmb_keycode(code: WinitKeycode) -> Keycode {
    macro_rules! codes {
        ($($code:ident),*) => {
            $(if code == WinitKeycode::$code {
                return Keycode::$code;
            })*
        };
    }

    #[rustfmt::skip]
    codes!(
        Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, Key0, A, B, C, D, E, F, G, H, I, J,
        K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Escape, F1, F2, F3, F4, F5, F6, F7, F8, F9,
        F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24, Snapshot,
        Scroll, Pause, Insert, Home, Delete, End, PageDown, PageUp, Left, Up, Right, Down, Back,
        Return, Space, Compose, Caret, Numlock, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
        Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd, NumpadDivide, NumpadDecimal,
        NumpadComma, NumpadEnter, NumpadEquals, NumpadMultiply, NumpadSubtract, AbntC1, AbntC2,
        Apostrophe, Apps, Asterisk, At, Ax, Backslash, Calculator, Capital, Colon, Comma, Convert,
        Equals, Grave, Kana, Kanji, LAlt, LBracket, LControl, LShift, LWin, Mail, MediaSelect,
        MediaStop, Minus, Mute, MyComputer, NavigateForward, NavigateBackward, NextTrack,
        NoConvert, OEM102, Period, PlayPause, Plus, Power, PrevTrack, RAlt, RBracket, RControl,
        RShift, RWin, Semicolon, Slash, Sleep, Stop, Sysrq, Tab, Underline, Unlabeled, VolumeDown,
        VolumeUp, Wake, WebBack, WebFavorites, WebForward, WebHome, WebRefresh, WebSearch, WebStop,
        Yen, Copy, Paste, Cut
    );

    panic!("unmatched keycode: {code:?}");
}

pub fn winit_to_pmb_key_state(state: WinitElementState) -> ElementState {
    match state {
        WinitElementState::Pressed => ElementState::Pressed,
        WinitElementState::Released => ElementState::Released,
    }
}

pub fn winit_to_pmb_mouse_button(button: WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Other(b) => MouseButton::Other(b as usize),
    }
}

#[derive(Clone, Copy)]
pub struct WgpuNdc {
    pub x: f32,
    pub y: f32,
}

impl std::fmt::Display for WgpuNdc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.02},{:.02}", self.x, self.y)
    }
}

pub fn pixel_to_ndc(width: u32, height: u32, pos: PixelPos) -> WgpuNdc {
    WgpuNdc {
        x: (2. * pos.x) / width as f32 - 1.,
        y: -((2. * pos.y) / height as f32 - 1.),
    }
}

pub fn ndc_to_pixel(width: u32, height: u32, pos: WgpuNdc) -> PixelPos {
    PixelPos {
        x: (pos.x + 1.) * width as f32 / 2.,
        y: (-pos.y + 1.) * height as f32 / 2.,
    }
}

pub fn ndc_to_stroke(width: u32, height: u32, zoom: f32, ndc: WgpuNdc) -> StrokePoint {
    StrokePoint {
        x: ndc.x * width as f32 / zoom,
        y: ndc.y * height as f32 / zoom,
    }
}

pub fn stroke_to_ndc(width: u32, height: u32, zoom: f32, point: StrokePoint) -> WgpuNdc {
    WgpuNdc {
        x: point.x * zoom / width as f32,
        y: point.y * zoom / height as f32,
    }
}

pub trait EventExt {
    fn is_input(&self) -> bool;
    fn is_window(&self) -> bool;
}

impl<T> EventExt for winit::event::Event<'_, T> {
    fn is_input(&self) -> bool {
        use winit::event::*;
        use DeviceEvent as D;
        use WindowEvent as W;

        matches!(
            self,
            Event::DeviceEvent {
                event: D::MouseMotion { .. }
                    | D::MouseWheel { .. }
                    | D::Motion { .. }
                    | D::Button { .. }
                    | D::Key(_)
                    | D::Text { .. },
                ..
            } | Event::WindowEvent {
                event: W::ReceivedCharacter(_)
                    | W::KeyboardInput { .. }
                    | W::ModifiersChanged(_)
                    | W::CursorMoved { .. }
                    | W::CursorEntered { .. }
                    | W::CursorLeft { .. }
                    | W::MouseWheel { .. }
                    | W::MouseInput { .. }
                    | W::TouchpadPressure { .. }
                    | W::AxisMotion { .. }
                    | W::Touch(_),
                ..
            }
        )
    }

    fn is_window(&self) -> bool {
        matches!(self, winit::event::Event::WindowEvent { .. })
    }
}

pub type Size = PhysicalSize<u32>;

pub struct Graphics {
    pub surface: Surface,
    pub surface_format: TextureFormat,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub size: Size,
    pub aa: bool,
    pub smaa_target: smaa::SmaaTarget,
    pub stroke_pipeline: RenderPipeline,
    pub stroke_view_bind_layout: BindGroupLayout,
    pub stroke_view_bind_group: BindGroup,
    pub stroke_view_uniform_buffer: Buffer,
    pub cursor_buffer: Buffer,
    pub cursor_pipeline: RenderPipeline,
    pub cursor_bind_layout: BindGroupLayout,
    pub cursor_bind_group: BindGroup,
    pub cursor_view_uniform_buffer: Buffer,
}

impl Graphics {
    pub async fn new(window: &Window) -> Self {
        log::info!("setting up wgpu");
        let size = window.inner_size();
        let instance = Instance::new(Backends::all());
        let surface = unsafe { instance.create_surface(window) };

        log::debug!("requesting adapter");
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let limits = Limits {
            max_push_constant_size: 128,
            ..Default::default()
        };

        log::debug!("requesting device");
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Features::PUSH_CONSTANTS,
                    limits,
                },
                None,
            )
            .await
            .unwrap();

        log::debug!("setting up pipeline stuff");
        let surface_format = surface.get_supported_formats(&adapter)[0];

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Immediate,
        };

        surface.configure(&device, &config);

        let stroke_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/stroke_line.wgsl"));

        let stroke_view_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("view bl"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let stroke_view_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("view ub"),
            contents: bytemuck::cast_slice(&glam::Mat4::IDENTITY.to_cols_array()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let stroke_view_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("view bg"),
            layout: &stroke_view_bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: stroke_view_uniform_buffer.as_entire_binding(),
            }],
        });

        let stroke_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&stroke_view_bind_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..12,
            }],
        });

        let cts = [Some(ColorTargetState {
            format: config.format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];

        let stroke_desc = RenderPipelineDescriptor {
            label: Some("stroke pipeline"),
            layout: Some(&stroke_layout),
            vertex: VertexState {
                module: &stroke_shader,
                entry_point: "vmain",
                buffers: &[
                    VertexBufferLayout {
                        array_stride: (size_of::<f32>() * 2) as BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x2,
                        }],
                    },
                    VertexBufferLayout {
                        array_stride: size_of::<f32>() as BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[VertexAttribute {
                            offset: 0,
                            shader_location: 1,
                            format: VertexFormat::Float32,
                        }],
                    },
                ],
            },
            fragment: Some(FragmentState {
                module: &stroke_shader,
                entry_point: "fmain",
                targets: &cts,
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        };

        let stroke_pipeline = device.create_render_pipeline(&stroke_desc);

        let cursor_points = powdermilk_biscuits::graphics::circle_points(1., NUM_SEGMENTS);

        let cursor_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("cursor points"),
            contents: bytemuck::cast_slice(cursor_points.as_slice()),
            usage: BufferUsages::VERTEX,
        });

        let cursor_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/cursor.wgsl"));

        let cursor_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("cursor"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                visibility: ShaderStages::VERTEX,
                count: None,
            }],
        });

        let cursor_view_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&glam::Mat4::IDENTITY.to_cols_array()),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let cursor_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("cursor"),
            bind_group_layouts: &[&cursor_bind_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..8,
            }],
        });

        let cursor_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &cursor_bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: cursor_view_uniform_buffer.as_entire_binding(),
            }],
        });

        let cursor_desc = RenderPipelineDescriptor {
            label: Some("cursor pipeline"),
            layout: Some(&cursor_layout),
            vertex: VertexState {
                module: &cursor_shader,
                entry_point: "vmain",
                buffers: &[VertexBufferLayout {
                    array_stride: (size_of::<f32>() * 2) as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: VertexFormat::Float32x2,
                    }],
                }],
            },
            fragment: Some(FragmentState {
                module: &cursor_shader,
                entry_point: "fmain",
                targets: &cts,
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        };

        let cursor_pipeline = device.create_render_pipeline(&cursor_desc);

        log::debug!("creating smaa target");
        let smaa_target = smaa::SmaaTarget::new(
            &device,
            &queue,
            size.width,
            size.height,
            surface_format,
            smaa::SmaaMode::Smaa1X,
        );

        log::info!("done!");
        Graphics {
            surface,
            surface_format,
            device,
            queue,
            config,
            size,
            aa: true,
            smaa_target,
            stroke_pipeline,
            stroke_view_bind_layout,
            stroke_view_bind_group,
            stroke_view_uniform_buffer,
            cursor_buffer,
            cursor_pipeline,
            cursor_bind_layout,
            cursor_bind_group,
            cursor_view_uniform_buffer,
        }
    }

    pub fn resize(&mut self, new_size: Size) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.smaa_target = smaa::SmaaTarget::new(
                &self.device,
                &self.queue,
                new_size.width,
                new_size.height,
                self.surface_format,
                smaa::SmaaMode::Smaa1X,
            );
        }
    }

    pub fn buffer_stroke(&mut self, stroke: &mut Stroke<StrokeBackend>) {
        stroke.replace_backend_with(|points, pressure| StrokeBackend {
            points: self.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: points,
                usage: BufferUsages::VERTEX,
            }),
            pressure: self.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: pressure,
                usage: BufferUsages::VERTEX,
            }),
            dirty: false,
        });
    }

    pub fn buffer_all_strokes(&mut self, state: &mut WgslState) {
        for stroke in state.strokes.iter_mut() {
            if stroke.is_dirty() {
                self.buffer_stroke(stroke);
            }
        }
    }

    pub fn render(
        &mut self,
        state: &mut WgslState,
        size: PhysicalSize<u32>,
        cursor_visible: bool,
    ) -> Result<(), SurfaceError> {
        let stroke_view = view_matrix(state.zoom, state.zoom, size, state.origin);

        let cursor_view = view_matrix(
            state.zoom,
            state.brush_size as f32,
            size,
            state.stylus.point,
        );

        self.queue.write_buffer(
            &self.stroke_view_uniform_buffer,
            0,
            bytemuck::cast_slice(&stroke_view.to_cols_array()),
        );

        self.queue.write_buffer(
            &self.cursor_view_uniform_buffer,
            0,
            bytemuck::cast_slice(&cursor_view.to_cols_array()),
        );

        self.buffer_all_strokes(state);

        self.queue.submit(None);

        macro_rules! render {
            ($frame:expr, $end:expr) => {
                let mut encoder = self
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("encoder"),
                    });

                {
                    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("render pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: $frame,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(WgpuColor::BLACK),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    pass.set_pipeline(&self.stroke_pipeline);
                    pass.set_bind_group(0, &self.stroke_view_bind_group, &[]);

                    for stroke in state.strokes.iter() {
                        if stroke.erased() || stroke.points().is_empty() {
                            continue;
                        }

                        pass.set_push_constants(
                            ShaderStages::VERTEX,
                            0,
                            bytemuck::cast_slice(&stroke.color().to_float()),
                        );

                        pass.set_vertex_buffer(0, stroke.backend().unwrap().points.slice(..));
                        pass.set_vertex_buffer(1, stroke.backend().unwrap().pressure.slice(..));
                        pass.draw(0..stroke.points().len() as u32, 0..1);
                    }
                }

                if !cursor_visible {
                    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("cursor"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: $frame,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Load,
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    let info_buffer = [
                        if state.stylus.down() { 1.0f32 } else { 0. },
                        if state.stylus.inverted() { 1. } else { 0. },
                    ];

                    pass.set_pipeline(&self.cursor_pipeline);
                    pass.set_bind_group(0, &self.cursor_bind_group, &[]);
                    pass.set_push_constants(
                        ShaderStages::VERTEX,
                        0,
                        bytemuck::cast_slice(&info_buffer),
                    );
                    pass.set_vertex_buffer(0, self.cursor_buffer.slice(..));
                    pass.draw(0..(NUM_SEGMENTS + 1) as u32, 0..1);
                }

                self.queue.submit(Some(encoder.finish()));
                let _ = $end;
            };
        }

        let output = self.surface.get_current_texture()?;
        let surface_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        if self.aa {
            let smaa_frame = self
                .smaa_target
                .start_frame(&self.device, &self.queue, &surface_view);
            render!(&smaa_frame, {
                smaa_frame.resolve();
            });
        } else {
            render!(&surface_view, {});
        }

        output.present();

        Ok(())
    }
}
