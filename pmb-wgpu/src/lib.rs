use powdermilk_biscuits::{
    event::{PenInfo, Touch, TouchPhase},
    graphics::{Color as PmbColor, ColorExt, PixelPos, StrokePoint},
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
    SurfaceError, TextureUsages, TextureViewDescriptor, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexState, VertexStepMode,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        ElementState, MouseButton, PenInfo as WinitPenInfo, Touch as WinitTouch,
        TouchPhase as WinitTouchPhase, VirtualKeyCode,
    },
    window::Window,
};

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
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub size: Size,
    pub pipeline: RenderPipeline,
    pub view_bind_layout: BindGroupLayout,
    pub view_bind_group: BindGroup,
    pub view_uniform_buffer: Buffer,
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

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
        };

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/stroke_line.wgsl"));

        let view_bind_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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

        let view_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("view ub"),
            contents: bytemuck::cast_slice(&glam::Mat4::IDENTITY.to_cols_array()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let view_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("view bg"),
            layout: &view_bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_uniform_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[&view_bind_layout],
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

        let desc = RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
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
                module: &shader,
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

        let pipeline = device.create_render_pipeline(&desc);

        Graphics {
            surface,
            device,
            queue,
            config,
            size,
            pipeline,
            view_bind_layout,
            view_bind_group,
            view_uniform_buffer,
        }
    }

    pub fn resize(&mut self, new_size: Size) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
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
    ) -> Result<(), SurfaceError> {
        self.buffer_all_strokes(state);

        let output = self.surface.get_current_texture()?;
        let surface_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let PhysicalSize { width, height } = size;
        let xform = stroke_to_ndc(width, height, state.settings.zoom, state.settings.origin);
        let view = glam::Mat4::from_scale_rotation_translation(
            glam::vec3(
                state.settings.zoom / width as f32,
                state.settings.zoom / height as f32,
                1.0,
            ),
            glam::Quat::IDENTITY,
            glam::vec3(xform.x, xform.y, 0.0),
        );

        self.queue.write_buffer(
            &self.view_uniform_buffer,
            0,
            bytemuck::cast_slice(&view.to_cols_array()),
        );
        self.queue.submit(None);

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(WgpuColor::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.view_bind_group, &[]);

            for stroke in state.strokes.iter() {
                pass.set_push_constants(
                    ShaderStages::VERTEX,
                    0,
                    bytemuck::cast_slice(&stroke.color().to_float()),
                );

                pass.set_vertex_buffer(0, stroke.backend().unwrap().buffer.slice(..));
                pass.draw(0..stroke.points().len() as u32, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
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
