//! Demonstrates 2D lighting with normal maps, multiple point lights, and shadows.

use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::css::{BLUE, ORANGE};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const TEXTURE_SIZE: usize = 256;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight2d {
            color: Color::srgb(0.08, 0.1, 0.16),
            brightness: 1.0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, orbit_lights)
        .run();
}

#[derive(Component)]
struct LightOrbit {
    center: Vec2,
    radius: Vec2,
    speed: f32,
    phase: f32,
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let (albedo, normal_map) = make_surface_textures();
    let albedo = images.add(albedo);
    let normal_map = images.add(normal_map);

    commands.spawn((
        Sprite {
            image: albedo,
            normal_map: Some(normal_map.clone()),
            custom_size: Some(Vec2::new(1100.0, 700.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -5.0),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::from_size(Vec2::splat(180.0)))),
        MeshMaterial2d(materials.add(ColorMaterial {
            color: Color::srgb(0.8, 0.8, 0.9),
            normal_map: Some(normal_map),
            ..default()
        })),
        Transform::from_xyz(0.0, 170.0, -0.5),
    ));

    for x in [-260.0, 260.0] {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.32, 0.34, 0.4), Vec2::new(64.0, 280.0)),
            Transform::from_xyz(x, -20.0, -1.0),
            LightOccluder2d::rectangle(Vec2::new(32.0, 140.0)),
        ));
    }

    commands.spawn((
        PointLight2d {
            color: ORANGE.into(),
            intensity: 1.7,
            range: 430.0,
            height: 180.0,
            shadows_enabled: true,
        },
        Transform::from_xyz(-300.0, 180.0, 0.0),
        LightOrbit {
            center: Vec2::ZERO,
            radius: Vec2::new(340.0, 170.0),
            speed: 0.55,
            phase: 0.0,
        },
    ));

    commands.spawn((
        PointLight2d {
            color: BLUE.into(),
            intensity: 1.9,
            range: 390.0,
            height: 140.0,
            shadows_enabled: true,
        },
        Transform::from_xyz(300.0, -140.0, 0.0),
        LightOrbit {
            center: Vec2::new(0.0, -30.0),
            radius: Vec2::new(300.0, 210.0),
            speed: -0.42,
            phase: core::f32::consts::PI,
        },
    ));

    commands.spawn((
        Text2d::new("2D lighting: two point lights, normal maps, and dynamic shadows"),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(-450.0, 300.0, 10.0),
    ));
}

fn orbit_lights(
    time: Res<Time>,
    mut lights: Query<(&LightOrbit, &mut Transform), With<PointLight2d>>,
) {
    for (orbit, mut transform) in &mut lights {
        let angle = time.elapsed_secs() * orbit.speed + orbit.phase;
        transform.translation.x = orbit.center.x + angle.cos() * orbit.radius.x;
        transform.translation.y = orbit.center.y + angle.sin() * orbit.radius.y;
    }
}

fn make_surface_textures() -> (Image, Image) {
    let mut albedo = Vec::with_capacity(TEXTURE_SIZE * TEXTURE_SIZE * 4);
    let mut normal = Vec::with_capacity(TEXTURE_SIZE * TEXTURE_SIZE * 4);

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let u = x as f32 / TEXTURE_SIZE as f32;
            let v = y as f32 / TEXTURE_SIZE as f32;
            let checker = ((x / 32) + (y / 32)) % 2;
            let shade = if checker == 0 { 0.48 } else { 0.62 };

            let wave_x = (u * core::f32::consts::TAU * 8.0).cos();
            let wave_y = (v * core::f32::consts::TAU * 8.0).sin();
            let height_x = wave_x * wave_y * 0.12;
            let height_y = wave_y * wave_x * 0.12;
            let n = Vec3::new(-height_x, -height_y, 1.0).normalize();

            albedo.extend_from_slice(&[
                (shade * 255.0) as u8,
                (shade * 0.9 * 255.0) as u8,
                (shade * 0.75 * 255.0) as u8,
                255,
            ]);
            normal.extend_from_slice(&[
                ((n.x * 0.5 + 0.5) * 255.0) as u8,
                ((n.y * 0.5 + 0.5) * 255.0) as u8,
                ((n.z * 0.5 + 0.5) * 255.0) as u8,
                255,
            ]);
        }
    }

    let size = Extent3d {
        width: TEXTURE_SIZE as u32,
        height: TEXTURE_SIZE as u32,
        depth_or_array_layers: 1,
    };

    (
        Image::new(
            size,
            TextureDimension::D2,
            albedo,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ),
        Image::new(
            size,
            TextureDimension::D2,
            normal,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        ),
    )
}
