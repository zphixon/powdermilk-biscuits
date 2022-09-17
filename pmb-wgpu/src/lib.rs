use powdermilk_biscuits::{
    bytemuck,
    event::{ElementState, Keycode, MouseButton, PenInfo, Touch, TouchPhase},
    graphics::{ColorExt, PixelPos, StrokePoint},
    stroke::Stroke,
    ui::Ui,
    Sketch, Tool,
};
use std::mem::size_of;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferAddress, BufferBindingType,
    BufferUsages, Color as WgpuColor, ColorTargetState, ColorWrites, CommandEncoder,
    CommandEncoderDescriptor, Device, DeviceDescriptor, Face, Features, FragmentState, FrontFace,
    IndexFormat, Instance, Limits, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor,
    PolygonMode, PowerPreference, PresentMode, PrimitiveState, PrimitiveTopology,
    PushConstantRange, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, ShaderStages, Surface, SurfaceConfiguration,
    SurfaceError, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
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

pub type WgpuStroke = Stroke<WgpuStrokeBackend>;

const NUM_SEGMENTS: usize = 50;

#[derive(Debug, Default, Clone, Copy)]
pub struct WgpuCoords;

impl powdermilk_biscuits::CoordinateSystem for WgpuCoords {
    type Ndc = WgpuNdc;

    fn pixel_to_ndc(width: u32, height: u32, pos: PixelPos) -> Self::Ndc {
        WgpuNdc {
            x: (2. * pos.x) / width as f32 - 1.,
            y: -((2. * pos.y) / height as f32 - 1.),
        }
    }

    fn ndc_to_pixel(width: u32, height: u32, pos: Self::Ndc) -> PixelPos {
        PixelPos {
            x: (pos.x + 1.) * width as f32 / 2.,
            y: (-pos.y + 1.) * height as f32 / 2.,
        }
    }

    fn ndc_to_stroke(width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint {
        StrokePoint {
            x: ndc.x * width as f32 / zoom,
            y: ndc.y * height as f32 / zoom,
        }
    }

    fn stroke_to_ndc(width: u32, height: u32, zoom: f32, point: StrokePoint) -> Self::Ndc {
        stroke_to_ndc(width, height, zoom, point)
    }
}

pub fn stroke_to_ndc(width: u32, height: u32, zoom: f32, point: StrokePoint) -> WgpuNdc {
    WgpuNdc {
        x: point.x * zoom / width as f32,
        y: point.y * zoom / height as f32,
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
pub struct WgpuStrokeBackend {
    pub points: Buffer,
    pub points_len: usize,
    pub mesh: Buffer,
    pub indices: Buffer,
    pub num_indices: usize,
    pub dirty: bool,
}

impl powdermilk_biscuits::StrokeBackend for WgpuStrokeBackend {
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

struct StrokeRenderer {
    triangle_pipeline: RenderPipeline,
    line_pipeline: RenderPipeline,
    view_bind_group: BindGroup,
    view_uniform_buffer: Buffer,
}

impl StrokeRenderer {
    fn new(device: &Device, format: TextureFormat) -> Self {
        let line_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/stroke_line.wgsl"));
        let mesh_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/stroke_mesh.wgsl"));

        let view_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("stroke bind layout"),
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

        let view_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("stroke view uniform buffer"),
            contents: bytemuck::cast_slice(&glam::Mat4::IDENTITY.to_cols_array()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let view_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("stroke view bind group"),
            layout: &view_bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("stroke pipeline layout"),
            bind_group_layouts: &[&view_bind_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..12,
            }],
        });

        let cts = [Some(ColorTargetState {
            format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];

        let triangle_pipeline_desc = RenderPipelineDescriptor {
            label: Some("stroke mesh pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &mesh_shader,
                entry_point: "vmain",
                buffers: &[VertexBufferLayout {
                    array_stride: (size_of::<f32>() * 2) as BufferAddress,
                    attributes: &[VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: VertexFormat::Float32x2,
                    }],
                    step_mode: VertexStepMode::Vertex,
                }],
            },
            fragment: Some(FragmentState {
                module: &mesh_shader,
                entry_point: "fmain",
                targets: &cts,
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
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

        let line_pipeline_desc = RenderPipelineDescriptor {
            label: Some("stroke line pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &line_shader,
                entry_point: "vmain",
                buffers: &[VertexBufferLayout {
                    array_stride: (size_of::<f32>() * 3) as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x2,
                        },
                        VertexAttribute {
                            offset: (size_of::<f32>() * 2) as u64,
                            shader_location: 1,
                            format: VertexFormat::Float32,
                        },
                    ],
                }],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &line_shader,
                entry_point: "fmain",
                targets: &cts,
            }),
            multiview: None,
        };

        let triangle_pipeline = device.create_render_pipeline(&triangle_pipeline_desc);
        let line_pipeline = device.create_render_pipeline(&line_pipeline_desc);

        StrokeRenderer {
            triangle_pipeline,
            line_pipeline,
            view_bind_group,
            view_uniform_buffer,
        }
    }

    fn render(
        &self,
        queue: &Queue,
        frame: &TextureView,
        encoder: &mut CommandEncoder,
        sketch: &Sketch<WgpuStrokeBackend>,
        size: Size,
    ) {
        let stroke_view = view_matrix(sketch.zoom, sketch.zoom, size, sketch.origin);
        queue.write_buffer(
            &self.view_uniform_buffer,
            0,
            bytemuck::cast_slice(&stroke_view.to_cols_array()),
        );

        queue.submit(None);

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: frame,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(WgpuColor::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        sketch.visible_strokes().for_each(|stroke| {
            if stroke.draw_tesselated {
                pass.set_pipeline(&self.triangle_pipeline);
            } else {
                pass.set_pipeline(&self.line_pipeline);
            }

            pass.set_bind_group(0, &self.view_bind_group, &[]);
            pass.set_push_constants(
                ShaderStages::VERTEX,
                0,
                bytemuck::cast_slice(&stroke.color().to_float()),
            );

            if stroke.draw_tesselated {
                let WgpuStrokeBackend {
                    mesh,
                    indices,
                    num_indices,
                    ..
                } = stroke.backend().unwrap();
                pass.set_vertex_buffer(0, mesh.slice(..));
                pass.set_index_buffer(indices.slice(..), IndexFormat::Uint16);
                pass.draw_indexed(0..(*num_indices as u32), 0, 0..1);
            } else {
                let WgpuStrokeBackend {
                    points, points_len, ..
                } = stroke.backend().unwrap();
                pass.set_vertex_buffer(0, points.slice(..));
                pass.draw(0..(*points_len as u32), 0..1);
            }
        });
    }
}

struct CursorRenderer {
    vertex_buffer: Buffer,
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    view_uniform_buffer: Buffer,
    pen_state_uniform_buffer: Buffer,
}

impl CursorRenderer {
    fn new(device: &Device, format: TextureFormat) -> Self {
        let cursor_points = powdermilk_biscuits::graphics::cursor_geometry(1., NUM_SEGMENTS);

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("cursor points"),
            contents: bytemuck::cast_slice(cursor_points.as_slice()),
            usage: BufferUsages::VERTEX,
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/cursor.wgsl"));

        let bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("cursor bind layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    // TODO separate bind group layouts?
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                },
            ],
        });

        let view_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("cursor uniform buffer"),
            contents: bytemuck::cast_slice(&glam::Mat4::IDENTITY.to_cols_array()),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let pen_state_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("cursor pen state buffer"),
            contents: bytemuck::cast_slice(&[0.0, 0.0]),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("cursor pipeline layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..8,
            }],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("cursor bind group"),
            layout: &bind_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: pen_state_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let cts = [Some(ColorTargetState {
            format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];

        let pipeline_desc = RenderPipelineDescriptor {
            label: Some("cursor pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
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
                module: &shader,
                entry_point: "fmain",
                targets: &cts,
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineList,
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

        let pipeline = device.create_render_pipeline(&pipeline_desc);

        CursorRenderer {
            vertex_buffer,
            pipeline,
            bind_group,
            view_uniform_buffer,
            pen_state_uniform_buffer,
        }
    }

    fn render(
        &self,
        queue: &Queue,
        frame: &TextureView,
        encoder: &mut CommandEncoder,
        ui: &Ui<WgpuCoords>,
        zoom: f32,
        size: Size,
    ) {
        let cursor_view = view_matrix(zoom, ui.brush_size as f32, size, ui.stylus.point);
        let info_buffer = [
            if ui.stylus.down() { 1.0f32 } else { 0. },
            if ui.active_tool == Tool::Eraser {
                1.
            } else {
                0.
            },
        ];

        queue.write_buffer(
            &self.view_uniform_buffer,
            0,
            bytemuck::cast_slice(&cursor_view.to_cols_array()),
        );

        queue.write_buffer(
            &self.pen_state_uniform_buffer,
            0,
            bytemuck::cast_slice(&info_buffer),
        );

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("cursor render pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: frame,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..(NUM_SEGMENTS * 2) as u32, 0..1);
    }
}

pub type Size = PhysicalSize<u32>;

pub struct Graphics {
    surface: Surface,
    surface_format: TextureFormat,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    pub size: Size,
    pub aa: bool,
    smaa_target: smaa::SmaaTarget,
    stroke_renderer: StrokeRenderer,
    cursor_renderer: CursorRenderer,
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
            max_push_constant_size: adapter.limits().max_push_constant_size,
            ..Default::default()
        };

        log::debug!("requesting device");
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("device descriptor"),
                    features: Features::PUSH_CONSTANTS,
                    limits,
                },
                None,
            )
            .await
            .unwrap();

        log::debug!("setting up pipeline stuff");
        let formats = surface.get_supported_formats(&adapter);

        let surface_format = if formats.contains(&TextureFormat::Rgba8UnormSrgb) {
            TextureFormat::Rgba8UnormSrgb
        } else {
            formats[0]
        };

        let present_mode = if surface
            .get_supported_modes(&adapter)
            .contains(&PresentMode::Immediate)
        {
            PresentMode::Immediate
        } else {
            PresentMode::Fifo
        };

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
        };

        surface.configure(&device, &config);

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
            stroke_renderer: StrokeRenderer::new(&device, surface_format),
            cursor_renderer: CursorRenderer::new(&device, surface_format),

            surface,
            surface_format,
            device,
            queue,
            config,
            size,
            aa: true,
            smaa_target,
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

    pub fn buffer_stroke(&mut self, stroke: &mut Stroke<WgpuStrokeBackend>) {
        stroke.backend.replace({
            WgpuStrokeBackend {
                points: self.device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("points buffer"),
                    contents: bytemuck::cast_slice(&stroke.points),
                    usage: BufferUsages::VERTEX,
                }),
                points_len: stroke.points.len(),
                mesh: self.device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("mesh buffer"),
                    contents: bytemuck::cast_slice(&stroke.mesh.vertices),
                    usage: BufferUsages::VERTEX,
                }),
                indices: self.device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("index buffer"),
                    contents: bytemuck::cast_slice(&stroke.mesh.indices),
                    usage: BufferUsages::INDEX,
                }),
                num_indices: stroke.mesh.indices.len(),
                dirty: false,
            }
        });
    }

    pub fn buffer_all_strokes(&mut self, sketch: &mut Sketch<WgpuStrokeBackend>) {
        for stroke in sketch.strokes.values_mut() {
            if stroke.is_dirty() {
                self.buffer_stroke(stroke);
            }
        }
    }

    pub fn render(
        &mut self,
        sketch: &mut Sketch<WgpuStrokeBackend>,
        ui: &Ui<WgpuCoords>,
        size: PhysicalSize<u32>,
        cursor_visible: bool,
    ) -> Result<(), SurfaceError> {
        self.buffer_all_strokes(sketch);

        macro_rules! render {
            ($frame:expr) => {
                let mut encoder = self
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("encoder"),
                    });

                self.stroke_renderer
                    .render(&self.queue, $frame, &mut encoder, sketch, size);

                if !cursor_visible {
                    self.cursor_renderer.render(
                        &self.queue,
                        $frame,
                        &mut encoder,
                        ui,
                        sketch.zoom,
                        size,
                    );
                }

                self.queue.submit(Some(encoder.finish()));
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

            render!(&smaa_frame);

            smaa_frame.resolve();
        } else {
            render!(&surface_view);
        }

        output.present();

        Ok(())
    }
}
