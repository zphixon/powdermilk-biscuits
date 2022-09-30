use powdermilk_biscuits::{
    bytemuck, egui,
    graphics::{PixelPos, StrokePoint},
    stroke::Stroke,
    ui::widget::SketchWidget,
    winit::{
        self,
        dpi::{PhysicalPosition, PhysicalSize},
        window::Window,
    },
    CoordinateSystem, Sketch, Tool,
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

pub type WgpuStroke = Stroke<WgpuStrokeBackend>;

const NUM_SEGMENTS: usize = 50;

#[derive(Debug, Default, Clone, Copy)]
pub struct WgpuCoords;

impl CoordinateSystem for WgpuCoords {
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
        WgpuNdc {
            x: point.x * zoom / width as f32,
            y: point.y * zoom / height as f32,
        }
    }
}

pub fn view_matrix(
    zoom: f32,
    scale: f32,
    size: PhysicalSize<u32>,
    origin: StrokePoint,
) -> glam::Mat4 {
    let PhysicalSize { width, height } = size;
    let xform = WgpuCoords::stroke_to_ndc(width, height, zoom, origin);
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
        bg_color: [f32; 3],
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
                    load: LoadOp::Clear(WgpuColor {
                        r: bg_color[0] as f64,
                        g: bg_color[1] as f64,
                        b: bg_color[2] as f64,
                        a: 1.,
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        sketch.visible_strokes().for_each(|stroke| {
            pass.set_pipeline(&self.line_pipeline);

            pass.set_bind_group(0, &self.view_bind_group, &[]);
            pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::cast_slice(&stroke.color));

            let WgpuStrokeBackend {
                points, points_len, ..
            } = stroke.backend().unwrap();
            pass.set_vertex_buffer(0, points.slice(..));
            pass.draw(0..(*points_len as u32), 0..1);

            if stroke.draw_tesselated {
                pass.set_pipeline(&self.triangle_pipeline);

                pass.set_bind_group(0, &self.view_bind_group, &[]);
                pass.set_push_constants(
                    ShaderStages::VERTEX,
                    0,
                    bytemuck::cast_slice(&stroke.color),
                );

                let WgpuStrokeBackend {
                    mesh,
                    indices,
                    num_indices,
                    ..
                } = stroke.backend().unwrap();
                pass.set_vertex_buffer(0, mesh.slice(..));
                pass.set_index_buffer(indices.slice(..), IndexFormat::Uint16);
                pass.draw_indexed(0..(*num_indices as u32), 0, 0..1);
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
        widget: &SketchWidget<WgpuCoords>,
        zoom: f32,
        size: Size,
    ) {
        let cursor_view = view_matrix(zoom, widget.brush_size as f32, size, widget.stylus.point);
        let info_buffer = [
            if widget.stylus.down() { 1.0f32 } else { 0. },
            if widget.active_tool == Tool::Eraser {
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
    pub surface: Surface,
    pub surface_format: TextureFormat,
    pub device: Device,
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

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        sketch: &mut Sketch<WgpuStrokeBackend>,
        widget: &SketchWidget<WgpuCoords>,
        size: PhysicalSize<u32>,
        cursor_visible: bool,
        egui_tris: &[egui::ClippedPrimitive],
        egui_textures: &egui::TexturesDelta,
        egui_painter: &mut egui_wgpu::Renderer,
    ) -> Result<(), SurfaceError> {
        self.buffer_all_strokes(sketch);

        macro_rules! render {
            ($frame:expr) => {
                let mut encoder = self
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("encoder"),
                    });

                self.stroke_renderer.render(
                    &self.queue,
                    $frame,
                    &mut encoder,
                    sketch,
                    size,
                    sketch.bg_color,
                );

                if !cursor_visible {
                    self.cursor_renderer.render(
                        &self.queue,
                        $frame,
                        &mut encoder,
                        widget,
                        sketch.zoom,
                        size,
                    );
                }

                for (id, image) in &egui_textures.set {
                    egui_painter.update_texture(&self.device, &self.queue, *id, image);
                }
                let sd = egui_wgpu::renderer::ScreenDescriptor {
                    size_in_pixels: [size.width, size.height],
                    pixels_per_point: 1.,
                };
                egui_painter.update_buffers(&self.device, &self.queue, egui_tris, &sd);
                egui_painter.render(&mut encoder, $frame, egui_tris, &sd, None);
                for id in &egui_textures.free {
                    egui_painter.free_texture(id);
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
