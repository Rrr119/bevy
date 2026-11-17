---
title: 2D lighting with normal maps, multiple lights, and shadows
authors: ["@Rrr119"]
pull_requests: []
---

Bevy's sprite and 2D mesh renderers now support forward 2D lighting without an
extra camera or render pass.

- `Sprite::normal_map` adds per-pixel surface detail. `ColorMaterial` also has a
  `normal_map` field for `Mesh2d` surfaces.
- Any number of `PointLight2d` components can affect a 2D view, up to
  `MAX_LIGHTS_2D`.
- `LightOccluder2d` polygons and polylines cast dynamic shadows from point
  lights.
- `AmbientLight2d` controls global illumination, and custom `Material2d` shaders
  can reuse `package::light2d::lighting`.

The default ambient light is white at full brightness, so existing scenes keep
their original appearance until lighting is configured.
