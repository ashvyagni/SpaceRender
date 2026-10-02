use std::sync::Arc;

use bevy_ecs::prelude::*;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use cosmogon_core::math::Vec3d;
use cosmogon_ecs::components::camera::*;
use cosmogon_ecs::components::celestial::*;
use cosmogon_ecs::components::transform::*;
use cosmogon_ecs::resources::global_time::*;
use cosmogon_ecs::resources::input_state::*;
use cosmogon_ecs::resources::render_state::*;
use cosmogon_physics::systems as physics_systems;
use cosmogon_render::loading_screen::LoadingScreen;
use cosmogon_render::renderer::Renderer;
use cosmogon_render::systems as render_systems;
use cosmogon_scene::loader;
use cosmogon_ui::systems::UiState;

use crate::config::AppConfig;
use crate::input::InputHandler;

enum AppState {
    Loading,
    Running,
}

struct OrbitState {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: glam::Vec3,
}

impl Default for OrbitState {
    fn default() -> Self {
        Self {
            yaw: 0.3,
            pitch: 0.4,
            distance: 200.0,
            target: glam::Vec3::ZERO,
        }
    }
}

impl OrbitState {
    fn camera_eye(&self) -> glam::Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.target + glam::Vec3::new(x, y, z)
    }
}

pub struct CosmogonApp {
    world: World,
    schedule: Schedule,
    renderer: Option<Renderer>,
    loading_screen: Option<LoadingScreen>,
    orbit: OrbitState,
    state: AppState,
    window: Option<Arc<Window>>,
    _input_handler: InputHandler,
    _config: AppConfig,
}

impl CosmogonApp {
    pub fn new() -> Self {
        let config = AppConfig::default();

        let mut world = World::default();

        world.insert_resource(GlobalTime::default());
        world.insert_resource(RenderConfig::default());
        world.insert_resource(FrameStats::default());
        world.insert_resource(SelectedObject::default());
        world.insert_resource(MouseState::default());
        world.insert_resource(KeyboardState::default());
        world.insert_resource(ScrollDelta::default());
        world.insert_resource(physics_systems::GravityConfig::default());
        world.insert_resource(render_systems::RenderQueue::default());
        world.insert_resource(UiState::default());

        let mut schedule = Schedule::default();
        schedule.add_systems((
            physics_systems::propagate_orbits,
            physics_systems::compute_gravity,
            physics_systems::integrate_motion,
            render_systems::build_transforms,
        ));
        schedule.add_systems(physics_systems::update_physics_time);

        Self {
            world,
            schedule,
            renderer: None,
            loading_screen: None,
            orbit: OrbitState::default(),
            state: AppState::Loading,
            window: None,
            _input_handler: InputHandler::new(),
            _config: config,
        }
    }

    fn spawn_solar_system(&mut self) {
        let bodies = loader::load_default_scene();
        let body_count = bodies.len();
        let mut commands = self.world.commands();
        for body in bodies {
            body.spawn(&mut commands);
        }

        // Spawn camera entity (position will be overridden by orbit camera)
        commands.spawn((
            Position {
                coords: Vec3d::new(0.0, 0.0, 0.0),
            },
            Camera {
                fov: 50.0_f32.to_radians(),
                near: 0.01,
                far: 50000.0,
                aspect: 1920.0 / 1080.0,
            },
            CameraTarget(Entity::PLACEHOLDER),
            CameraDistance(200.0),
            CameraState {
                speed: 2.0,
                sensitivity: 0.003,
            },
            OrbitCamera,
        ));

        self.world.flush();

        log::info!("Solar system spawned with {} bodies", body_count);
    }

    pub fn run(self) {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        let mut app = self;
        event_loop.run_app(&mut app).expect("Event loop failed");
    }
}

impl ApplicationHandler for CosmogonApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = WindowAttributes::default()
                .with_title("Cosmogon — Universe Simulator")
                .with_inner_size(winit::dpi::LogicalSize::new(1920, 1080));

            let window = Arc::new(
                event_loop
                    .create_window(attrs)
                    .expect("Failed to create window"),
            );

            let renderer = Renderer::new(window.clone());
            let loading_screen = LoadingScreen::new(&renderer.gpu.device, renderer.gpu.surface_format());

            self.renderer = Some(renderer);
            self.loading_screen = Some(loading_screen);
            self.window = Some(window);

            self.spawn_solar_system();
            log::info!("Cosmogon initialized — loading screen active");

            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                if matches!(self.state, AppState::Running) {
                    let dragging = self.world.resource::<MouseState>().left_pressed;

                    if dragging {
                        self.orbit.yaw += delta.0 as f32 * 0.005;
                        self.orbit.pitch += delta.1 as f32 * 0.005;
                        self.orbit.pitch = self.orbit.pitch.clamp(-1.5, 1.5);
                    }
                }

                let mut mouse = self.world.resource_mut::<MouseState>();
                mouse.delta = (mouse.delta.0 + delta.0 as f32, mouse.delta.1 + delta.1 as f32);
            }
            _ => {}
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(new_size.width, new_size.height);
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::event::ElementState::Pressed = event.state {
                    if matches!(self.state, AppState::Running) {
                        match event.logical_key.as_ref() {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => {
                                let mut time = self.world.resource_mut::<GlobalTime>();
                                time.paused = !time.paused;
                            }
                            winit::keyboard::Key::Character("1") => {
                                self.world.resource_mut::<GlobalTime>().acceleration = 1.0;
                            }
                            winit::keyboard::Key::Character("2") => {
                                self.world.resource_mut::<GlobalTime>().acceleration = 10.0;
                            }
                            winit::keyboard::Key::Character("3") => {
                                self.world.resource_mut::<GlobalTime>().acceleration = 100.0;
                            }
                            winit::keyboard::Key::Character("4") => {
                                self.world.resource_mut::<GlobalTime>().acceleration = 1000.0;
                            }
                            winit::keyboard::Key::Character("5") => {
                                self.world.resource_mut::<GlobalTime>().acceleration = 10000.0;
                            }
                            _ => {}
                        }
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let mut mouse = self.world.resource_mut::<MouseState>();
                match state {
                    winit::event::ElementState::Pressed => match button {
                        winit::event::MouseButton::Left => { mouse.left_pressed = true; }
                        winit::event::MouseButton::Right => { mouse.right_pressed = true; }
                        winit::event::MouseButton::Middle => { mouse.middle_pressed = true; }
                        _ => {}
                    },
                    winit::event::ElementState::Released => match button {
                        winit::event::MouseButton::Left => { mouse.left_pressed = false; }
                        winit::event::MouseButton::Right => { mouse.right_pressed = false; }
                        winit::event::MouseButton::Middle => { mouse.middle_pressed = false; }
                        _ => {}
                    },
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let mut mouse = self.world.resource_mut::<MouseState>();
                mouse.position = (position.x as f32, position.y as f32);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if matches!(self.state, AppState::Running) {
                    let scroll_amount = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };
                    self.orbit.distance -= scroll_amount * 15.0;
                    self.orbit.distance = self.orbit.distance.clamp(20.0, 10000.0);
                }

                let mut scroll = self.world.resource_mut::<ScrollDelta>();
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => { scroll.0 += y; }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => { scroll.0 += pos.y as f32; }
                }
            }

            WindowEvent::RedrawRequested => {
                let frame_start = std::time::Instant::now();

                match &mut self.state {
                    AppState::Loading => {
                        if let (Some(renderer), Some(loading_screen)) =
                            (&mut self.renderer, &mut self.loading_screen)
                        {
                            let frame = match renderer.gpu.surface.get_current_texture() {
                                Ok(frame) => frame,
                                Err(wgpu::SurfaceError::Lost) => {
                                    let (w, h) = (
                                        renderer.gpu.surface_config.width,
                                        renderer.gpu.surface_config.height,
                                    );
                                    renderer.gpu.resize(w, h);
                                    if let Some(window) = &self.window {
                                        window.request_redraw();
                                    }
                                    return;
                                }
                                Err(e) => {
                                    log::warn!("Surface error during loading: {e:?}");
                                    if let Some(window) = &self.window {
                                        window.request_redraw();
                                    }
                                    return;
                                }
                            };

                            let view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let mut encoder = renderer.gpu.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("Loading Screen Encoder"),
                                },
                            );

                            loading_screen.render(
                                &mut encoder,
                                &view,
                                &renderer.gpu.device,
                                &renderer.gpu.queue,
                            );

                            renderer.gpu.queue.submit(std::iter::once(encoder.finish()));
                            frame.present();

                            if loading_screen.is_done() {
                                log::info!("Loading complete — transitioning to universe");
                                self.state = AppState::Running;
                            }
                        }
                    }
                    AppState::Running => {
                        // 1. Advance time
                        {
                            let mut time = self.world.resource_mut::<GlobalTime>();
                            time.frame_count += 1;
                            if !time.paused {
                                time.delta = 1.0 / 60.0 * time.acceleration;
                                time.elapsed += time.delta;
                            }
                        }

                        // 2. Run physics + transform build
                        self.schedule.run(&mut self.world);

                        // 3. Collect render data
                        let time_val = self.world.resource::<GlobalTime>().elapsed as f32;
                        let entries: Vec<render_systems::RenderEntry> =
                            self.world.resource::<render_systems::RenderQueue>().entries.clone();

                        // Find Sun position for orbit target + light
                        let scale = render_systems::AU_RENDER_SCALE as f32;
                        let mut sun_render_pos = glam::Vec3::ZERO;
                        {
                            let mut pos_q = self.world.query::<(&Position, Entity)>();
                            for (pos, entity) in pos_q.iter(&self.world) {
                                if self.world.get::<Star>(entity).is_some() {
                                    sun_render_pos = glam::Vec3::new(
                                        pos.coords.x as f32 * scale,
                                        pos.coords.y as f32 * scale,
                                        pos.coords.z as f32 * scale,
                                    );
                                    break;
                                }
                            }
                        }

                        // Update orbit camera target to follow the Sun
                        self.orbit.target = sun_render_pos;

                        // Compute camera from orbit state
                        let cam_eye = self.orbit.camera_eye();
                        let cam_fov = 50.0_f32.to_radians();
                        let cam_near = 0.01_f32;
                        let cam_far = 50000.0_f32;

                        let aspect = if let Some(renderer) = &self.renderer {
                            renderer.gpu.surface_config.width as f32
                                / renderer.gpu.surface_config.height as f32
                        } else {
                            1920.0 / 1080.0
                        };

                        // 4. Render
                        if let Some(renderer) = &mut self.renderer {
                            renderer.sync_scene(&entries);
                            renderer.set_camera(
                                cam_eye, sun_render_pos, glam::Vec3::Y,
                                aspect, cam_fov, cam_near, cam_far, time_val,
                            );
                            renderer.set_light(sun_render_pos, glam::Vec3::new(1.0, 0.95, 0.8));
                            renderer.render(time_val);
                        }

                        // 5. Update stats
                        let frame_time = frame_start.elapsed();
                        {
                            let mut stats = self.world.resource_mut::<FrameStats>();
                            stats.frame_time_ms = frame_time.as_secs_f64() * 1000.0;
                            stats.fps = 1.0 / frame_time.as_secs_f64();
                        }

                        // 6. Reset per-frame input
                        self.world.resource_mut::<MouseState>().delta = (0.0, 0.0);
                    }
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}
