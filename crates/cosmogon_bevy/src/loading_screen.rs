use bevy::prelude::*;

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_loading_screen)
           .add_systems(Update, update_loading_screen);
    }
}

#[derive(Component)]
struct LoadingScreen;

#[derive(Component)]
struct LoadingText;

#[derive(Component)]
struct LoadingProgressBar;

#[derive(Component)]
struct LoadingProgressFill;

fn spawn_loading_screen(mut commands: Commands) {
    // Create loading screen UI
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
        LoadingScreen,
    )).with_children(|parent| {
        // Title
        parent.spawn((
            Text::new("COSMOGON"),
            TextFont {
                font_size: 72.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.9, 0.7)),
            Node {
                margin: UiRect::bottom(Val::Px(20.0)),
                ..default()
            },
        ));

        // Subtitle
        parent.spawn((
            Text::new("Universe Simulator"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            Node {
                margin: UiRect::bottom(Val::Px(40.0)),
                ..default()
            },
        ));

        // Loading text
        parent.spawn((
            Text::new("Loading..."),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
            Node {
                margin: UiRect::bottom(Val::Px(20.0)),
                ..default()
            },
            LoadingText,
        ));

        // Progress bar background
        parent.spawn((
            Node {
                width: Val::Px(400.0),
                height: Val::Px(20.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.4, 0.4, 0.4)),
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            LoadingProgressBar,
        )).with_children(|parent| {
            // Progress bar fill
            parent.spawn((
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.6, 1.0)),
                LoadingProgressFill,
            ));
        });
    });
}

fn update_loading_screen(
    time: Res<Time>,
    mut fill_query: Query<&mut Node, With<LoadingProgressFill>>,
    mut text_query: Query<&mut Text, With<LoadingText>>,
) {
    // Simulate loading progress
    let progress = (time.elapsed_secs() / 3.0).min(1.0); // 3 second loading
    
    for mut node in &mut fill_query {
        node.width = Val::Percent(progress * 100.0);
    }
    
    for mut text in &mut text_query {
        **text = format!("Loading... {}%", (progress * 100.0) as i32);
    }
}
