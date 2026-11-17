//! 2D lighting for sprites and 2D meshes.
//!
//! The lighting model is forward rendered. Sprites and [`ColorMaterial`] meshes
//! use their normal maps directly in their fragment shaders, so no extra lighting
//! render pass or camera is required. Lights are additive and shadows are
//! calculated from [`LightOccluder2d`] line segments.
//!
//! [`ColorMaterial`]: crate::ColorMaterial

use bevy_app::{App, Plugin};
use bevy_color::{Color, LinearRgba};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_math::{UVec4, Vec2, Vec4};
use bevy_reflect::prelude::*;
use bevy_render::{
    render_resource::{ShaderType, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
};
use bevy_shader::load_shader_library;
use bevy_transform::components::{GlobalTransform, Transform};

/// The maximum number of [`PointLight2d`]s that can affect a 2D view.
pub const MAX_LIGHTS_2D: usize = 32;

/// The maximum number of shadow-casting line segments in a 2D view.
pub const MAX_LIGHT_OCCLUDER_SEGMENTS_2D: usize = 128;

/// A point light for 2D scenes.
///
/// Add a [`Transform`] to position the light. Any number of lights can affect
/// the same sprite or mesh, up to [`MAX_LIGHTS_2D`].
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_sprite_render::PointLight2d;
/// # use bevy_transform::components::Transform;
/// # use bevy_color::palettes::css::ORANGE;
/// # fn setup(mut commands: Commands) {
/// commands.spawn((
///     PointLight2d {
///         color: ORANGE.into(),
///         intensity: 1.5,
///         range: 400.0,
///         ..Default::default()
///     },
///     Transform::from_xyz(100.0, 50.0, 0.0),
/// ));
/// # }
/// ```
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(Transform)]
pub struct PointLight2d {
    /// The color of the light.
    pub color: Color,
    /// The brightness multiplier applied to [`PointLight2d::color`].
    pub intensity: f32,
    /// The maximum distance at which the light contributes.
    pub range: f32,
    /// The height of the light above the 2D plane.
    ///
    /// This controls the direction of the light relative to surface normals.
    /// Larger values make normal-mapped surfaces face the light more directly.
    pub height: f32,
    /// Whether this light is blocked by [`LightOccluder2d`]s.
    pub shadows_enabled: bool,
}

impl Default for PointLight2d {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            range: 200.0,
            height: 100.0,
            shadows_enabled: true,
        }
    }
}

/// Global ambient light for 2D scenes.
///
/// The default is white with a brightness of `1.0`, which keeps scenes without
/// any [`PointLight2d`]s unchanged.
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource, Default, Debug, Clone)]
pub struct AmbientLight2d {
    /// The ambient light color.
    pub color: Color,
    /// The brightness multiplier applied to [`AmbientLight2d::color`].
    pub brightness: f32,
}

impl Default for AmbientLight2d {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            brightness: 1.0,
        }
    }
}

/// A polygonal 2D light occluder used to cast shadows.
///
/// The vertices are in the entity's local space. Consecutive vertices form line
/// segments, and if [`LightOccluder2d::closed`] is `true`, the last vertex is
/// connected back to the first one.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(Transform)]
pub struct LightOccluder2d {
    /// Vertices of the occluder polygon or polyline.
    pub vertices: Vec<Vec2>,
    /// Whether the last vertex connects back to the first vertex.
    pub closed: bool,
}

impl Default for LightOccluder2d {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            closed: true,
        }
    }
}

impl LightOccluder2d {
    /// Creates a closed polygon occluder from a list of local-space vertices.
    pub fn polygon(vertices: impl Into<Vec<Vec2>>) -> Self {
        Self {
            vertices: vertices.into(),
            closed: true,
        }
    }

    /// Creates an open polyline occluder from a list of local-space vertices.
    pub fn polyline(vertices: impl Into<Vec<Vec2>>) -> Self {
        Self {
            vertices: vertices.into(),
            closed: false,
        }
    }

    /// Creates a rectangular occluder centered on the entity's origin.
    pub fn rectangle(half_extents: Vec2) -> Self {
        Self::polygon([
            Vec2::new(-half_extents.x, -half_extents.y),
            Vec2::new(half_extents.x, -half_extents.y),
            Vec2::new(half_extents.x, half_extents.y),
            Vec2::new(-half_extents.x, half_extents.y),
        ])
    }
}

#[derive(Clone, Copy, Debug, Default, ShaderType)]
struct GpuPointLight2d {
    /// `xy` is the position, `z` is the range, and `w` is the intensity.
    position_range_intensity: Vec4,
    /// `rgb` is the color and `w` is reserved for future shadow parameters.
    color: Vec4,
    /// `x` is the light height and `y` indicates whether shadows are enabled.
    params: Vec4,
}

/// GPU representation of all 2D lights for a view.
#[derive(Clone, Debug, ShaderType)]
pub(crate) struct Lighting2dUniform {
    /// `rgb` is the ambient color multiplied by its brightness.
    ambient: Vec4,
    /// `x` is the light count and `y` is the occluder segment count.
    counts: UVec4,
    lights: [GpuPointLight2d; MAX_LIGHTS_2D],
    occluders: [Vec4; MAX_LIGHT_OCCLUDER_SEGMENTS_2D],
}

impl Default for Lighting2dUniform {
    fn default() -> Self {
        Self {
            ambient: Vec4::ZERO,
            counts: UVec4::ZERO,
            lights: [GpuPointLight2d::default(); MAX_LIGHTS_2D],
            occluders: [Vec4::ZERO; MAX_LIGHT_OCCLUDER_SEGMENTS_2D],
        }
    }
}

/// GPU lighting data shared by all 2D views.
#[derive(Resource, Default)]
pub struct Light2dUniforms {
    pub(crate) uniforms: UniformBuffer<Lighting2dUniform>,
}

#[derive(Resource, Default)]
struct ExtractedLights2d {
    ambient: LinearRgba,
    brightness: f32,
    lights: Vec<GpuPointLight2d>,
    occluders: Vec<Vec4>,
}

/// Adds 2D point lights, ambient lighting, normal-map lighting, and shadows.
pub struct Light2dPlugin;

impl Plugin for Light2dPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "lighting.wesl");

        app.register_type::<PointLight2d>()
            .register_type::<AmbientLight2d>()
            .register_type::<LightOccluder2d>()
            .init_resource::<AmbientLight2d>();

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedLights2d>()
                .init_resource::<Light2dUniforms>()
                .add_systems(ExtractSchedule, extract_lights_2d)
                .add_systems(
                    Render,
                    prepare_light2d_uniforms.in_set(RenderSystems::PrepareResources),
                );
        }
    }
}

fn extract_lights_2d(
    mut extracted: ResMut<ExtractedLights2d>,
    ambient: Extract<Res<AmbientLight2d>>,
    lights: Extract<Query<(Entity, &PointLight2d, &GlobalTransform)>>,
    occluders: Extract<Query<(Entity, &LightOccluder2d, &GlobalTransform)>>,
) {
    extracted.ambient = ambient.color.to_linear();
    extracted.brightness = ambient.brightness.max(0.0);
    extracted.lights.clear();
    extracted.occluders.clear();

    let mut lights: Vec<(Entity, PointLight2d, Vec2)> = lights
        .iter()
        .map(|(entity, light, transform)| (entity, *light, transform.translation().truncate()))
        .collect();
    lights.sort_unstable_by_key(|(entity, _, _)| entity.index());

    for (_, light, position) in lights.into_iter().take(MAX_LIGHTS_2D) {
        let color = light.color.to_linear();
        extracted.lights.push(GpuPointLight2d {
            position_range_intensity: Vec4::new(
                position.x,
                position.y,
                light.range.max(0.0),
                light.intensity.max(0.0),
            ),
            color: Vec4::new(color.red, color.green, color.blue, 1.0),
            params: Vec4::new(
                light.height,
                if light.shadows_enabled { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ),
        });
    }

    let mut occluders: Vec<(Entity, LightOccluder2d, GlobalTransform)> = occluders
        .iter()
        .map(|(entity, occluder, transform)| (entity, occluder.clone(), *transform))
        .collect();
    occluders.sort_unstable_by_key(|(entity, _, _)| entity.index());

    'occluders: for (_, occluder, transform) in occluders {
        if occluder.vertices.len() < 2 {
            continue;
        }

        let mut push_segment = |a: Vec2, b: Vec2| -> bool {
            if extracted.occluders.len() == MAX_LIGHT_OCCLUDER_SEGMENTS_2D {
                return false;
            }
            let a = transform.transform_point(a.extend(0.0)).truncate();
            let b = transform.transform_point(b.extend(0.0)).truncate();
            extracted.occluders.push(Vec4::new(a.x, a.y, b.x, b.y));
            true
        };

        for vertices in occluder.vertices.windows(2) {
            if !push_segment(vertices[0], vertices[1]) {
                break 'occluders;
            }
        }

        if occluder.closed
            && occluder.vertices.len() > 2
            && let (Some(first), Some(last)) = (occluder.vertices.first(), occluder.vertices.last())
            && !push_segment(*last, *first)
        {
            break;
        }
    }
}

fn prepare_light2d_uniforms(
    mut light2d_uniforms: ResMut<Light2dUniforms>,
    extracted: Res<ExtractedLights2d>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let mut data = Lighting2dUniform::default();
    let ambient = extracted.ambient;
    data.ambient = Vec4::new(
        ambient.red * extracted.brightness,
        ambient.green * extracted.brightness,
        ambient.blue * extracted.brightness,
        1.0,
    );
    data.counts = UVec4::new(
        extracted.lights.len() as u32,
        extracted.occluders.len() as u32,
        0,
        0,
    );

    for (index, light) in extracted.lights.iter().enumerate() {
        data.lights[index] = *light;
    }
    for (index, occluder) in extracted.occluders.iter().enumerate() {
        data.occluders[index] = *occluder;
    }

    light2d_uniforms.uniforms.set(data);
    light2d_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_occluder_helpers() {
        let rectangle = LightOccluder2d::rectangle(Vec2::splat(4.0));
        assert_eq!(rectangle.vertices.len(), 4);
        assert!(rectangle.closed);

        let polyline = LightOccluder2d::polyline([Vec2::ZERO, Vec2::X]);
        assert_eq!(polyline.vertices.len(), 2);
        assert!(!polyline.closed);
    }

    #[test]
    fn ambient_default_keeps_unlit_scenes_unchanged() {
        let ambient = AmbientLight2d::default();
        assert_eq!(ambient.color, Color::WHITE);
        assert_eq!(ambient.brightness, 1.0);
    }
}
