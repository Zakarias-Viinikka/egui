use bevy::prelude::*;
use bevy::render::camera::ClearColorConfig;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts};

const GROWTH_SPEED: f32 = 75.0;
const MAX_DT: f32 = 0.05;
const SEGMENTS: usize = 512; // Increased to 512 for ultra-smooth circles

#[derive(Resource, Default)]
pub struct AppState {
    pub paused: bool,
}

#[derive(Resource)]
pub struct Circles {
    pub phase: f32,
}

impl Default for Circles {
    fn default() -> Self {
        Self { phase: 0.0 }
    }
}

pub fn setup(mut commands: Commands, mut config_store: ResMut<GizmoConfigStore>) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Msaa::Sample4,
    ));

    // Configure gizmos for maximum smoothness
    let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.line_width = 2.5;
    // Ensure smooth line joins (no jagged corners)
    config.line_joints = GizmoLineJoint::Round(8);

    commands.init_resource::<AppState>();
    commands.init_resource::<Circles>();
}

pub fn update_circles(
    time: Res<Time>,
    state: Res<AppState>,
    mut circles: ResMut<Circles>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if state.paused {
        return;
    }
    let Ok(window) = windows.get_single() else {
        return;
    };
    if !window.focused {
        return;
    }

    let max_radius =
        (window.width().powi(2) + window.height().powi(2)).sqrt() / 2.0;
    if max_radius <= 0.0 {
        return;
    }

    let dt = time.delta_secs().min(MAX_DT);
    circles.phase = (circles.phase + GROWTH_SPEED * dt / max_radius) % 1.0;
}

pub fn draw_circles(circles: Res<Circles>, windows: Query<&Window, With<PrimaryWindow>>, mut gizmos: Gizmos) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let max_radius =
        (window.width().powi(2) + window.height().powi(2)).sqrt() / 2.0;
    
    let radius1 = circles.phase * max_radius;
    let radius2 = ((circles.phase + 0.5) % 1.0) * max_radius;

    if radius1 > 0.0 {
        draw_smooth_circle(&mut gizmos, radius1, Color::srgb(1.0, 0.2, 0.2));
    }
    if radius2 > 0.0 {
        draw_smooth_circle(&mut gizmos, radius2, Color::srgb(0.2, 0.8, 1.0));
    }
}

fn draw_smooth_circle(gizmos: &mut Gizmos, radius: f32, color: Color) {
    let points: Vec<Vec2> = (0..=SEGMENTS)
        .map(|i| {
            let angle = std::f32::consts::TAU * (i as f32 / SEGMENTS as f32);
            Vec2::new(angle.cos() * radius, angle.sin() * radius)
        })
        .collect();
    gizmos.linestrip_2d(points, color);
}

pub fn ui_system(mut contexts: EguiContexts, mut state: ResMut<AppState>) {
    let ctx = contexts.ctx_mut();

    let transparent = egui::Frame::none()
        .fill(egui::Color32::TRANSPARENT)
        .inner_margin(16.0);

    egui::CentralPanel::default()
        .frame(transparent)
        .show(ctx, |ui| {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                let button_text = if state.paused { "▶ Play" } else { "⏸ Pause" };
                if ui.button(button_text).clicked() {
                    state.paused = !state.paused;
                }
            });
        });
}