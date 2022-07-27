use powdermilk_biscuits::{
    event::{PenInfo, Touch, TouchPhase},
    graphics::{ColorExt, PixelPos, StrokePoint},
    stroke::{Stroke, StrokeElement},
    State,
};
use std::{collections::HashMap, mem::size_of};
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
        ElementState, MouseButton, PenInfo as WinitPenInfo, Touch as WinitTouch,
        TouchPhase as WinitTouchPhase, VirtualKeyCode,
    },
    window::Window,
};

const NUM_SEGMENTS: usize = 50;

#[derive(Debug, Default)]
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
    pub buffer: Buffer,
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

pub fn glutin_to_pmb_touch(touch: WinitTouch) -> Touch {
    Touch {
        force: touch.force.map(|f| f.normalized()),
        phase: glutin_to_pmb_touch_phase(touch.phase),
        location: physical_pos_to_pixel_pos(touch.location),
        pen_info: touch.pen_info.map(glutin_to_pmb_pen_info),
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

pub type Size = PhysicalSize<u32>;

pub struct Graphics {
    pub surface: Surface,
    pub surface_format: TextureFormat,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub size: Size,
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
        let size = window.inner_size();
        let instance = Instance::new(Backends::all());
        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let mut limits = Limits::default();
        limits.max_push_constant_size = 128;

        // segfault on linux here :(
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

        let surface_format = surface.get_supported_formats(&adapter)[0];

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
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
                buffers: &[VertexBufferLayout {
                    array_stride: size_of::<StrokeElement>() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x2,
                        },
                        VertexAttribute {
                            offset: 2 * size_of::<f32>() as u64,
                            shader_location: 1,
                            format: VertexFormat::Float32,
                        },
                    ],
                }],
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

        let smaa_target = smaa::SmaaTarget::new(
            &device,
            &queue,
            size.width,
            size.height,
            surface_format,
            smaa::SmaaMode::Smaa1X,
        );

        Graphics {
            surface,
            surface_format,
            device,
            queue,
            config,
            size,
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
        stroke.replace_backend_with(|bytes| StrokeBackend {
            buffer: self.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytes,
                usage: BufferUsages::VERTEX,
            }),
            dirty: false,
        });
    }

    pub fn buffer_all_strokes(&mut self, state: &mut State<WgpuBackend, StrokeBackend>) {
        for stroke in state.strokes.iter_mut() {
            if stroke.is_dirty() {
                self.buffer_stroke(stroke);
            }
        }
    }

    pub fn render(
        &mut self,
        state: &mut State<WgpuBackend, StrokeBackend>,
        size: PhysicalSize<u32>,
        cursor_visible: bool,
    ) -> Result<(), SurfaceError> {
        let stroke_view = view_matrix(
            state.settings.zoom,
            state.settings.zoom,
            size,
            state.settings.origin,
        );

        let cursor_view = view_matrix(
            state.settings.zoom,
            state.settings.brush_size as f32,
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

        let output = self.surface.get_current_texture()?;
        let surface_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
        let smaa_frame = self
            .smaa_target
            .start_frame(&self.device, &self.queue, &surface_view);

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &smaa_frame,
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

                pass.set_vertex_buffer(0, stroke.backend().unwrap().buffer.slice(..));
                pass.draw(0..stroke.points().len() as u32, 0..1);
            }
        }

        if !cursor_visible {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("cursor"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &smaa_frame,
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
            pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::cast_slice(&info_buffer));
            pass.set_vertex_buffer(0, self.cursor_buffer.slice(..));
            pass.draw(0..(NUM_SEGMENTS + 1) as u32, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));

        smaa_frame.resolve();
        output.present();

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum KeyState {
    Downstroke,
    Held,
    Released,
}

impl KeyState {
    pub fn is_down(&self) -> bool {
        use KeyState::*;
        matches!(self, Downstroke | Held)
    }

    pub fn just_pressed(&self) -> bool {
        use KeyState::*;
        matches!(self, Downstroke)
    }
}

#[derive(Default)]
pub struct InputHandler {
    keys: HashMap<VirtualKeyCode, KeyState>,
    buttons: HashMap<MouseButton, KeyState>,
    cursor_pos: PhysicalPosition<f64>,
}

fn cycle_state(key_state: KeyState, element_state: ElementState) -> KeyState {
    match (key_state, element_state) {
        (KeyState::Released, ElementState::Pressed) => KeyState::Downstroke,
        (_, ElementState::Released) => KeyState::Released,
        (_, ElementState::Pressed) => KeyState::Held,
    }
}

impl InputHandler {
    pub fn handle_mouse_move(&mut self, cursor_pos: PhysicalPosition<f64>) {
        self.cursor_pos = cursor_pos;
    }

    pub fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        let button_state = self.buttons.entry(button).or_insert(KeyState::Released);
        let next_state = cycle_state(*button_state, state);
        *button_state = next_state;
    }

    pub fn cursor_pos(&self) -> PhysicalPosition<f64> {
        self.cursor_pos
    }

    pub fn button_down(&mut self, button: MouseButton) -> bool {
        self.buttons.contains_key(&button) && self.buttons[&button].is_down()
    }

    pub fn button_just_pressed(&mut self, button: MouseButton) -> bool {
        self.buttons.contains_key(&button) && self.buttons[&button].just_pressed()
    }

    pub fn handle_key(&mut self, key: VirtualKeyCode, state: ElementState) {
        let key_state = self.keys.entry(key).or_insert(KeyState::Released);
        let next_state = cycle_state(*key_state, state);
        *key_state = next_state;
    }

    pub fn is_down(&self, key: VirtualKeyCode) -> bool {
        self.keys.contains_key(&key) && self.keys[&key].is_down()
    }

    pub fn just_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keys.contains_key(&key) && self.keys[&key].just_pressed()
    }

    pub fn shift(&self) -> bool {
        use VirtualKeyCode::{LShift, RShift};
        self.is_down(LShift) || self.is_down(RShift)
    }

    pub fn control(&self) -> bool {
        use VirtualKeyCode::{LControl, RControl};
        self.is_down(LControl) || self.is_down(RControl)
    }
}
