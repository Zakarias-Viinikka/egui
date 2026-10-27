use bevy::prelude::*;
use bevy::window::{PresentMode, WindowPlugin};
use bevy_egui::EguiPlugin;

use crate::bevy_setup;

pub fn run() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Immediate,
                ..default()
            }),
            ..default()
        })
    );

    app.add_plugins(EguiPlugin);

    app.add_systems(Startup, bevy_setup::setup);
    
    app.add_systems(
        Update, 
        (
            bevy_setup::update_circles,
            bevy_setup::draw_circles,
            bevy_setup::ui_system,
        )
    );

    app.run();
}