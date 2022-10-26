use cosmic_text::{Attrs, FontSystem, TextBuffer, TextMetrics};
use glyphon::{Color, HasColor, Resolution, TextAtlas, TextRenderer};
use wgpu::{
    Backends, CommandEncoderDescriptor, CompositeAlphaMode, DeviceDescriptor, Features, Instance,
    Limits, LoadOp, Operations, PresentMode, RenderPassColorAttachment, RenderPassDescriptor,
    RequestAdapterOptions, SurfaceConfiguration, TextureUsages, TextureViewDescriptor,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

fn main() {
    pollster::block_on(run());
}

#[derive(Clone, Copy)]
struct GlyphUserData;

impl HasColor for GlyphUserData {
    fn color(&self) -> Color {
        Color {
            r: 255,
            g: 255,
            b: 0,
            a: 255,
        }
    }
}

static mut FONT_SYSTEM: Option<FontSystem> = None;

async fn run() {
    let instance = Instance::new(Backends::all());
    let adapter = instance
        .request_adapter(&RequestAdapterOptions::default())
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                features: Features::empty(),
                limits: Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let surface = unsafe { instance.create_surface(&window) };
    let size = window.inner_size();
    let swapchain_format = surface.get_supported_formats(&adapter)[0];
    let mut config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: PresentMode::Mailbox,
        alpha_mode: CompositeAlphaMode::Opaque,
    };
    surface.configure(&device, &config);

    unsafe {
        FONT_SYSTEM = Some(FontSystem::new());
    }
    let mut atlas = TextAtlas::new(&device, &queue, swapchain_format);
    let mut text_renderer = TextRenderer::new(&device, &queue);

    let mut buffer = TextBuffer::new(
        unsafe { FONT_SYSTEM.as_ref().unwrap() },
        Attrs::new(),
        TextMetrics::new(32, 44),
    );
    buffer.set_size(800, 600);
    buffer.set_text(include_str!("./ligature.txt"));
    buffer.shape_until_cursor();

    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter);

        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                config.width = size.width;
                config.height = size.height;
                surface.configure(&device, &config);
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                text_renderer
                    .prepare(
                        &device,
                        &queue,
                        &mut atlas,
                        Resolution {
                            width: config.width,
                            height: config.height,
                        },
                        &mut buffer,
                    )
                    .unwrap();

                let frame = surface.get_current_texture().unwrap();
                let view = frame.texture.create_view(&TextureViewDescriptor::default());
                let mut encoder =
                    device.create_command_encoder(&CommandEncoderDescriptor { label: None });
                {
                    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(wgpu::Color::BLACK),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    text_renderer.render(&atlas, &mut pass).unwrap();
                }

                queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
