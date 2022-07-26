use crate::{
    event::{PenInfo, Touch, TouchPhase},
    graphics::{PixelPos, StrokePoint},
};
use std::collections::HashMap;
use wgpu::{
    Backends, BlendState, Buffer, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
    Device, DeviceDescriptor, Face, Features, FragmentState, FrontFace, Instance, Limits, LoadOp,
    MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PowerPreference,
    PresentMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, Surface,
    SurfaceConfiguration, SurfaceError, TextureUsages, TextureViewDescriptor, VertexState,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        ElementState, MouseButton, PenInfo as WinitPenInfo, Touch as WinitTouch,
        TouchPhase as WinitTouchPhase, VirtualKeyCode,
    },
    window::Window,
};

#[derive(Debug)]
pub struct StrokeBackend {
    pub buffer: Option<Buffer>,
}

impl From<WinitPenInfo> for PenInfo {
    fn from(pen_info: WinitPenInfo) -> Self {
        PenInfo {
            barrel: pen_info.barrel,
            inverted: pen_info.inverted,
            eraser: pen_info.eraser,
        }
    }
}

impl From<WinitTouchPhase> for TouchPhase {
    fn from(phase: WinitTouchPhase) -> Self {
        match phase {
            WinitTouchPhase::Started => TouchPhase::Start,
            WinitTouchPhase::Moved => TouchPhase::Move,
            WinitTouchPhase::Ended => TouchPhase::End,
            WinitTouchPhase::Cancelled => TouchPhase::Cancel,
        }
    }
}

impl From<WinitTouch> for Touch {
    fn from(touch: WinitTouch) -> Self {
        Touch {
            force: touch.force.map(|force| force.normalized()),
            phase: touch.phase.into(),
            location: touch.location.into(),
            pen_info: touch.pen_info.map(|pen_info| pen_info.into()),
        }
    }
}

impl From<PhysicalPosition<f64>> for PixelPos {
    fn from(pp: PhysicalPosition<f64>) -> Self {
        PixelPos {
            x: pp.x as f32,
            y: pp.y as f32,
        }
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

        // segfault on linux here :(
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Features::empty(),
                    limits: Limits::default(),
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

        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/triangle.wgsl"));

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
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
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fmain",
                targets: &cts,
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
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

    pub fn render(&mut self, state: &crate::State) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: state.stylus.pressure as f64,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.draw(0..3, 0..1);
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
