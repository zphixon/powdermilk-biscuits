use crate::{
    event::{PenInfo, Touch, TouchPhase},
    graphics::{PixelPos, StrokePoint},
};
use wgpu::{
    Backends, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    Limits, LoadOp, Operations, PowerPreference, PresentMode, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, Surface, SurfaceConfiguration, SurfaceError,
    TextureUsages, TextureViewDescriptor,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{PenInfo as WinitPenInfo, Touch as WinitTouch, TouchPhase as WinitTouchPhase},
    window::Window,
};

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

        Graphics {
            surface,
            device,
            queue,
            config,
            size,
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

    pub fn render(&mut self) -> Result<(), SurfaceError> {
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
            let _rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }
}
