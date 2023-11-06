//! A module for the [`Gizmos`] [`SystemParam`].

use std::{f32::consts::TAU, iter, marker::PhantomData};

use bevy_ecs::{
    component::Tick,
    system::{Deferred, ReadOnlySystemParam, Res, Resource, SystemBuffer, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use bevy_math::{Mat2, Quat, Vec2, Vec3};
use bevy_render::color::Color;
use bevy_transform::TransformPoint;

use crate::{
    config::CustomGizmoConfig,
    config::{DefaultGizmoConfig, GizmoConfigStore},
    prelude::GizmoConfig,
};

type PositionItem = [f32; 3];
type ColorItem = [f32; 4];

const DEFAULT_CIRCLE_SEGMENTS: usize = 32;

#[derive(Resource, Default)]
pub(crate) struct GizmoStorage<T: CustomGizmoConfig> {
    pub list_positions: Vec<PositionItem>,
    pub list_colors: Vec<ColorItem>,
    pub strip_positions: Vec<PositionItem>,
    pub strip_colors: Vec<ColorItem>,
    marker: PhantomData<T>,
}

/// A [`SystemParam`] for drawing gizmos.
///
/// They are drawn in immediate mode, which means they will be rendered only for
/// the frames in which they are spawned.
/// Gizmos should be spawned before the [`Last`](bevy_app::Last) schedule to ensure they are drawn.
pub struct Gizmos<'w, 's, T: CustomGizmoConfig = DefaultGizmoConfig> {
    buffer: Deferred<'s, GizmoBuffer<T>>,
    //_store: Res<'w, GizmoConfigStore>,
    /// The currently used [`GizmoConfig`]
    pub config: &'w GizmoConfig,
    /// The currently used [`CustomGizmoConfig`]
    pub config_ext: &'w T,
}

const _: () = {
    #[doc(hidden)]
    pub struct FetchState<T: CustomGizmoConfig> {
        state: <(
            Deferred<'static, GizmoBuffer<T>>,
            Res<'static, GizmoConfigStore>,
        ) as SystemParam>::State,
    }
    unsafe impl<T: CustomGizmoConfig> SystemParam for Gizmos<'_, '_, T> {
        type State = FetchState<T>;
        type Item<'w, 's> = Gizmos<'w, 's, T>;
        fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
            FetchState {
                state: <(
                    Deferred<'static, GizmoBuffer<T>>,
                    Res<'static, GizmoConfigStore>,
                ) as SystemParam>::init_state(world, system_meta),
            }
        }
        fn new_archetype(
            state: &mut Self::State,
            archetype: &bevy_ecs::archetype::Archetype,
            system_meta: &mut SystemMeta,
        ) {
            <(
                Deferred<'static, GizmoBuffer<T>>,
                Res<'static, GizmoConfigStore>,
            ) as SystemParam>::new_archetype(&mut state.state, archetype, system_meta);
        }
        fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
            <(
                Deferred<'static, GizmoBuffer<T>>,
                Res<'static, GizmoConfigStore>,
            ) as SystemParam>::apply(&mut state.state, system_meta, world);
        }
        unsafe fn get_param<'w, 's>(
            state: &'s mut Self::State,
            system_meta: &SystemMeta,
            world: UnsafeWorldCell<'w>,
            change_tick: Tick,
        ) -> Self::Item<'w, 's> {
            let (f0, f1) = <(
                Deferred<'static, GizmoBuffer<T>>,
                Res<'static, GizmoConfigStore>,
            ) as SystemParam>::get_param(
                &mut state.state, system_meta, world, change_tick
            );
            // Accessing the GizmoConfigStore in the immediate mode API reduces performance significantly.
            // Implementing SystemParam manually allows us to do it to here
            // Having config available allows for early returns when gizmos are disabled
            let (config, config_ext) = f1.into_inner().get::<T>();
            Gizmos {
                buffer: f0,
                //_store: f1,
                config,
                config_ext,
            }
        }
    }
    unsafe impl<'w, 's, T: CustomGizmoConfig> ReadOnlySystemParam for Gizmos<'w, 's, T>
    where
        Deferred<'s, GizmoBuffer<T>>: ReadOnlySystemParam,
        Res<'w, GizmoConfigStore>: ReadOnlySystemParam,
    {
    }
};

#[derive(Default)]
struct GizmoBuffer<T: CustomGizmoConfig> {
    list_positions: Vec<PositionItem>,
    list_colors: Vec<ColorItem>,
    strip_positions: Vec<PositionItem>,
    strip_colors: Vec<ColorItem>,
    marker: PhantomData<T>,
}

impl<T: CustomGizmoConfig> SystemBuffer for GizmoBuffer<T> {
    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        let mut storage = world.resource_mut::<GizmoStorage<T>>();
        storage.list_positions.append(&mut self.list_positions);
        storage.list_colors.append(&mut self.list_colors);
        storage.strip_positions.append(&mut self.strip_positions);
        storage.strip_colors.append(&mut self.strip_colors);
    }
}

impl<'w, 's, T: CustomGizmoConfig> Gizmos<'w, 's, T> {
    /// Draw a line in 3D from `start` to `end`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line(Vec3::ZERO, Vec3::X, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line(&mut self, start: Vec3, end: Vec3, color: Color) {
        if !self.config.enabled {
            return;
        }
        self.extend_list_positions([start, end]);
        self.add_list_color(color, 2);
    }

    /// Draw a line in 3D with a color gradient from `start` to `end`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient(Vec3::ZERO, Vec3::X, Color::GREEN, Color::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_gradient(&mut self, start: Vec3, end: Vec3, start_color: Color, end_color: Color) {
        if !self.config.enabled {
            return;
        }
        self.extend_list_positions([start, end]);
        self.extend_list_colors([start_color, end_color]);
    }

    /// Draw a line in 3D from `start` to `start + vector`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray(Vec3::Y, Vec3::X, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray(&mut self, start: Vec3, vector: Vec3, color: Color) {
        if !self.config.enabled {
            return;
        }
        self.line(start, start + vector, color);
    }

    /// Draw a line in 3D with a color gradient from `start` to `start + vector`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray_gradient(Vec3::Y, Vec3::X, Color::GREEN, Color::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_gradient(
        &mut self,
        start: Vec3,
        vector: Vec3,
        start_color: Color,
        end_color: Color,
    ) {
        if !self.config.enabled {
            return;
        }
        self.line_gradient(start, start + vector, start_color, end_color);
    }

    /// Draw a line in 3D made of straight segments between the points.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip([Vec3::ZERO, Vec3::X, Vec3::Y], Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip(&mut self, positions: impl IntoIterator<Item = Vec3>, color: Color) {
        if !self.config.enabled {
            return;
        }
        self.extend_strip_positions(positions);
        let len = self.buffer.strip_positions.len();
        self.buffer
            .strip_colors
            .resize(len - 1, color.as_linear_rgba_f32());
        self.buffer.strip_colors.push([f32::NAN; 4]);
    }

    /// Draw a line in 3D made of straight segments between the points, with a color gradient.
    ///
    /// This should be called for each frame the lines need to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_gradient([
    ///         (Vec3::ZERO, Color::GREEN),
    ///         (Vec3::X, Color::RED),
    ///         (Vec3::Y, Color::BLUE)
    ///     ]);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_gradient(&mut self, points: impl IntoIterator<Item = (Vec3, Color)>) {
        if !self.config.enabled {
            return;
        }
        let points = points.into_iter();

        let GizmoBuffer {
            strip_positions,
            strip_colors,
            ..
        } = &mut *self.buffer;

        let (min, _) = points.size_hint();
        strip_positions.reserve(min);
        strip_colors.reserve(min);

        for (position, color) in points {
            strip_positions.push(position.to_array());
            strip_colors.push(color.as_linear_rgba_f32());
        }

        strip_positions.push([f32::NAN; 3]);
        strip_colors.push([f32::NAN; 4]);
    }

    /// Draw a circle in 3D at `position` with the flat side facing `normal`.
    ///
    /// This should be called for each frame the circle needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.circle(Vec3::ZERO, Vec3::Z, 1., Color::GREEN);
    ///
    ///     // Circles have 32 line-segments by default.
    ///     // You may want to increase this for larger circles.
    ///     gizmos
    ///         .circle(Vec3::ZERO, Vec3::Z, 5., Color::RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn circle(
        &mut self,
        position: Vec3,
        normal: Vec3,
        radius: f32,
        color: Color,
    ) -> CircleBuilder<'_, 'w, 's, T> {
        CircleBuilder {
            gizmos: self,
            position,
            normal,
            radius,
            color,
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a wireframe sphere in 3D made out of 3 circles around the axes.
    ///
    /// This should be called for each frame the sphere needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.sphere(Vec3::ZERO, Quat::IDENTITY, 1., Color::BLACK);
    ///
    ///     // Each circle has 32 line-segments by default.
    ///     // You may want to increase this for larger spheres.
    ///     gizmos
    ///         .sphere(Vec3::ZERO, Quat::IDENTITY, 5., Color::BLACK)
    ///         .circle_segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn sphere(
        &mut self,
        position: Vec3,
        rotation: Quat,
        radius: f32,
        color: Color,
    ) -> SphereBuilder<'_, 'w, 's, T> {
        SphereBuilder {
            gizmos: self,
            position,
            rotation,
            radius,
            color,
            circle_segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a wireframe rectangle in 3D.
    ///
    /// This should be called for each frame the rectangle needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rect(Vec3::ZERO, Quat::IDENTITY, Vec2::ONE, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn rect(&mut self, position: Vec3, rotation: Quat, size: Vec2, color: Color) {
        if !self.config.enabled {
            return;
        }
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2.extend(0.));
        self.linestrip([tl, tr, br, bl, tl], color);
    }

    /// Draw a wireframe cube in 3D.
    ///
    /// This should be called for each frame the cube needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_transform::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.cuboid(Transform::IDENTITY, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn cuboid(&mut self, transform: impl TransformPoint, color: Color) {
        if !self.config.enabled {
            return;
        }
        let rect = rect_inner(Vec2::ONE);
        // Front
        let [tlf, trf, brf, blf] = rect.map(|vec2| transform.transform_point(vec2.extend(0.5)));
        // Back
        let [tlb, trb, brb, blb] = rect.map(|vec2| transform.transform_point(vec2.extend(-0.5)));

        let strip_positions = [
            tlf, trf, brf, blf, tlf, // Front
            tlb, trb, brb, blb, tlb, // Back
        ];
        self.linestrip(strip_positions, color);

        let list_positions = [
            trf, trb, brf, brb, blf, blb, // Front to back
        ];
        self.extend_list_positions(list_positions);
        self.add_list_color(color, 6);
    }

    /// Draw a line in 2D from `start` to `end`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_2d(Vec2::ZERO, Vec2::X, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_2d(&mut self, start: Vec2, end: Vec2, color: Color) {
        if !self.config.enabled {
            return;
        }
        self.line(start.extend(0.), end.extend(0.), color);
    }

    /// Draw a line in 2D with a color gradient from `start` to `end`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient_2d(Vec2::ZERO, Vec2::X, Color::GREEN, Color::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_gradient_2d(
        &mut self,
        start: Vec2,
        end: Vec2,
        start_color: Color,
        end_color: Color,
    ) {
        if !self.config.enabled {
            return;
        }
        self.line_gradient(start.extend(0.), end.extend(0.), start_color, end_color);
    }

    /// Draw a line in 2D made of straight segments between the points.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_2d([Vec2::ZERO, Vec2::X, Vec2::Y], Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_2d(&mut self, positions: impl IntoIterator<Item = Vec2>, color: Color) {
        if !self.config.enabled {
            return;
        }
        self.linestrip(positions.into_iter().map(|vec2| vec2.extend(0.)), color);
    }

    /// Draw a line in 2D made of straight segments between the points, with a color gradient.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_gradient_2d([
    ///         (Vec2::ZERO, Color::GREEN),
    ///         (Vec2::X, Color::RED),
    ///         (Vec2::Y, Color::BLUE)
    ///     ]);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_gradient_2d(&mut self, positions: impl IntoIterator<Item = (Vec2, Color)>) {
        if !self.config.enabled {
            return;
        }
        self.linestrip_gradient(
            positions
                .into_iter()
                .map(|(vec2, color)| (vec2.extend(0.), color)),
        );
    }

    /// Draw a line in 2D from `start` to `start + vector`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray_2d(Vec2::Y, Vec2::X, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_2d(&mut self, start: Vec2, vector: Vec2, color: Color) {
        if !self.config.enabled {
            return;
        }
        self.line_2d(start, start + vector, color);
    }

    /// Draw a line in 2D with a color gradient from `start` to `start + vector`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient(Vec3::Y, Vec3::X, Color::GREEN, Color::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_gradient_2d(
        &mut self,
        start: Vec2,
        vector: Vec2,
        start_color: Color,
        end_color: Color,
    ) {
        if !self.config.enabled {
            return;
        }
        self.line_gradient_2d(start, start + vector, start_color, end_color);
    }

    /// Draw a circle in 2D.
    ///
    /// This should be called for each frame the circle needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.circle_2d(Vec2::ZERO, 1., Color::GREEN);
    ///
    ///     // Circles have 32 line-segments by default.
    ///     // You may want to increase this for larger circles.
    ///     gizmos
    ///         .circle_2d(Vec2::ZERO, 5., Color::RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn circle_2d(
        &mut self,
        position: Vec2,
        radius: f32,
        color: Color,
    ) -> Circle2dBuilder<'_, 'w, 's, T> {
        Circle2dBuilder {
            gizmos: self,
            position,
            radius,
            color,
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw an arc, which is a part of the circumference of a circle, in 2D.
    ///
    /// This should be called for each frame the arc needs to be rendered.
    ///
    /// # Arguments
    /// - `position` sets the center of this circle.
    /// - `radius` controls the distance from `position` to this arc, and thus its curvature.
    /// - `direction_angle` sets the clockwise  angle in radians between `Vec2::Y` and
    /// the vector from `position` to the midpoint of the arc.
    /// - `arc_angle` sets the length of this arc, in radians.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use std::f32::consts::PI;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arc_2d(Vec2::ZERO, 0., PI / 4., 1., Color::GREEN);
    ///
    ///     // Arcs have 32 line-segments by default.
    ///     // You may want to increase this for larger arcs.
    ///     gizmos
    ///         .arc_2d(Vec2::ZERO, 0., PI / 4., 5., Color::RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn arc_2d(
        &mut self,
        position: Vec2,
        direction_angle: f32,
        arc_angle: f32,
        radius: f32,
        color: Color,
    ) -> Arc2dBuilder<'_, 'w, 's, T> {
        Arc2dBuilder {
            gizmos: self,
            position,
            direction_angle,
            arc_angle,
            radius,
            color,
            segments: None,
        }
    }

    /// Draw a wireframe rectangle in 2D.
    ///
    /// This should be called for each frame the rectangle needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rect_2d(Vec2::ZERO, 0., Vec2::ONE, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn rect_2d(&mut self, position: Vec2, rotation: f32, size: Vec2, color: Color) {
        if !self.config.enabled {
            return;
        }
        let rotation = Mat2::from_angle(rotation);
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2);
        self.linestrip_2d([tl, tr, br, bl, tl], color);
    }

    #[inline]
    fn extend_list_positions(&mut self, positions: impl IntoIterator<Item = Vec3>) {
        self.buffer
            .list_positions
            .extend(positions.into_iter().map(|vec3| vec3.to_array()));
    }

    #[inline]
    fn extend_list_colors(&mut self, colors: impl IntoIterator<Item = Color>) {
        self.buffer
            .list_colors
            .extend(colors.into_iter().map(|color| color.as_linear_rgba_f32()));
    }

    #[inline]
    fn add_list_color(&mut self, color: Color, count: usize) {
        self.buffer
            .list_colors
            .extend(iter::repeat(color.as_linear_rgba_f32()).take(count));
    }

    #[inline]
    fn extend_strip_positions(&mut self, positions: impl IntoIterator<Item = Vec3>) {
        self.buffer.strip_positions.extend(
            positions
                .into_iter()
                .map(|vec3| vec3.to_array())
                .chain(iter::once([f32::NAN; 3])),
        );
    }
}

/// A builder returned by [`Gizmos::circle`].
pub struct CircleBuilder<'a, 'w, 's, T: CustomGizmoConfig> {
    gizmos: &'a mut Gizmos<'w, 's, T>,
    position: Vec3,
    normal: Vec3,
    radius: f32,
    color: Color,
    segments: usize,
}

impl<T: CustomGizmoConfig> CircleBuilder<'_, '_, '_, T> {
    /// Set the number of line-segments for this circle.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<T: CustomGizmoConfig> Drop for CircleBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        if !self.gizmos.config.enabled {
            return;
        }
        let rotation = Quat::from_rotation_arc(Vec3::Z, self.normal);
        let positions = circle_inner(self.radius, self.segments)
            .map(|vec2| (self.position + rotation * vec2.extend(0.)));
        self.gizmos.linestrip(positions, self.color);
    }
}

/// A builder returned by [`Gizmos::sphere`].
pub struct SphereBuilder<'a, 'w, 's, T: CustomGizmoConfig> {
    gizmos: &'a mut Gizmos<'w, 's, T>,
    position: Vec3,
    rotation: Quat,
    radius: f32,
    color: Color,
    circle_segments: usize,
}

impl<T: CustomGizmoConfig> SphereBuilder<'_, '_, '_, T> {
    /// Set the number of line-segments per circle for this sphere.
    pub fn circle_segments(mut self, segments: usize) -> Self {
        self.circle_segments = segments;
        self
    }
}

impl<T: CustomGizmoConfig> Drop for SphereBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        if !self.gizmos.config.enabled {
            return;
        }
        for axis in Vec3::AXES {
            self.gizmos
                .circle(self.position, self.rotation * axis, self.radius, self.color)
                .segments(self.circle_segments);
        }
    }
}

/// A builder returned by [`Gizmos::circle_2d`].
pub struct Circle2dBuilder<'a, 'w, 's, T: CustomGizmoConfig> {
    gizmos: &'a mut Gizmos<'w, 's, T>,
    position: Vec2,
    radius: f32,
    color: Color,
    segments: usize,
}

impl<T: CustomGizmoConfig> Circle2dBuilder<'_, '_, '_, T> {
    /// Set the number of line-segments for this circle.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<T: CustomGizmoConfig> Drop for Circle2dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        if !self.gizmos.config.enabled {
            return;
        }
        let positions = circle_inner(self.radius, self.segments).map(|vec2| (vec2 + self.position));
        self.gizmos.linestrip_2d(positions, self.color);
    }
}

/// A builder returned by [`Gizmos::arc_2d`].
pub struct Arc2dBuilder<'a, 'w, 's, T: CustomGizmoConfig> {
    gizmos: &'a mut Gizmos<'w, 's, T>,
    position: Vec2,
    direction_angle: f32,
    arc_angle: f32,
    radius: f32,
    color: Color,
    segments: Option<usize>,
}

impl<T: CustomGizmoConfig> Arc2dBuilder<'_, '_, '_, T> {
    /// Set the number of line-segments for this arc.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = Some(segments);
        self
    }
}

impl<T: CustomGizmoConfig> Drop for Arc2dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        if !self.gizmos.config.enabled {
            return;
        }
        let segments = match self.segments {
            Some(segments) => segments,
            // Do a linear interpolation between 1 and `DEFAULT_CIRCLE_SEGMENTS`
            // using the arc angle as scalar.
            None => ((self.arc_angle.abs() / TAU) * DEFAULT_CIRCLE_SEGMENTS as f32).ceil() as usize,
        };

        let positions = arc_inner(self.direction_angle, self.arc_angle, self.radius, segments)
            .map(|vec2| (vec2 + self.position));
        self.gizmos.linestrip_2d(positions, self.color);
    }
}

fn arc_inner(
    direction_angle: f32,
    arc_angle: f32,
    radius: f32,
    segments: usize,
) -> impl Iterator<Item = Vec2> {
    (0..segments + 1).map(move |i| {
        let start = direction_angle - arc_angle / 2.;

        let angle = start + (i as f32 * (arc_angle / segments as f32));
        Vec2::from(angle.sin_cos()) * radius
    })
}

fn circle_inner(radius: f32, segments: usize) -> impl Iterator<Item = Vec2> {
    (0..segments + 1).map(move |i| {
        let angle = i as f32 * TAU / segments as f32;
        Vec2::from(angle.sin_cos()) * radius
    })
}

fn rect_inner(size: Vec2) -> [Vec2; 4] {
    let half_size = size / 2.;
    let tl = Vec2::new(-half_size.x, half_size.y);
    let tr = Vec2::new(half_size.x, half_size.y);
    let bl = Vec2::new(-half_size.x, -half_size.y);
    let br = Vec2::new(half_size.x, -half_size.y);
    [tl, tr, br, bl]
}
