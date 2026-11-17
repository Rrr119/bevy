# Bevy Sprite

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Crates.io](https://img.shields.io/crates/v/bevy_sprite_render.svg)](https://crates.io/crates/bevy_sprite_render)
[![Downloads](https://img.shields.io/crates/d/bevy_sprite_render.svg)](https://crates.io/crates/bevy_sprite_render)
[![Docs](https://docs.rs/bevy_sprite_render/badge.svg)](https://docs.rs/bevy_sprite_render/latest/bevy_sprite_render/)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

Provides 2D sprite and mesh rendering for Bevy.

## 2D Lighting

The sprite renderer includes forward-rendered 2D lighting that works with the
existing sprite and `Mesh2d` pipelines:

- Add a `normal_map` to a `Sprite`, or a `normal_map` to a `ColorMaterial`, to
  shade surface detail. Normal maps should use a linear texture format.
- Add any number of `PointLight2d` components, up to `MAX_LIGHTS_2D` per view.
- Add `LightOccluder2d` polygons or polylines to cast dynamic shadows.
- Configure global light with the `AmbientLight2d` resource. Its default is
  white at full brightness, so scenes without point lights look unchanged.

Sprite lighting is enabled by `SpriteRenderPlugin`; no extra camera or render
pass is required. Custom `Material2d` shaders can import the shared lighting
functions from `package::light2d::lighting`.

See the [`lighting_2d`](https://github.com/bevyengine/bevy/blob/main/examples/2d/lighting_2d.rs)
example for a complete scene.
