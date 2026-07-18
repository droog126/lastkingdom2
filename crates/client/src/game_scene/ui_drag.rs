//! Shared mouse dragging for the in-game UI panels.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Marks a panel whose position can be moved with its drag handle.
#[derive(Component)]
pub(crate) struct UiDragPanel;

/// Identifies the panel controlled by a title/drag handle.
#[derive(Component)]
pub(crate) struct UiDragHandle(pub(crate) Entity);

/// Tracks an in-progress drag. The panel's `UiTransform` is the source of truth
/// after the drag, so opening and closing a panel does not reset its position.
#[derive(Resource, Default)]
pub(crate) struct UiDragState {
    active_panel: Option<Entity>,
    last_cursor: Option<Vec2>,
    frontmost_z: i32,
}

pub(crate) fn drag_ui_panels(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut drag_state: ResMut<UiDragState>,
    handles: Query<(&Interaction, &UiDragHandle)>,
    mut panels: Query<
        (
            &mut UiTransform,
            &mut ZIndex,
            &ComputedNode,
            &UiGlobalTransform,
        ),
        With<UiDragPanel>,
    >,
) {
    let viewport = windows
        .single()
        .ok()
        .map(Window::size)
        .unwrap_or(Vec2::new(1280.0, 720.0));

    if !mouse.pressed(MouseButton::Left) {
        // A panel can be restored from a stale drag offset or be laid out
        // against a smaller window than the one it was created for. Keep all
        // draggable panels recoverable even when the user is not dragging.
        for (mut transform, _, computed, global) in &mut panels {
            let (_, _, current_center) = global.to_scale_angle_translation();
            let delta = clamp_drag_delta(current_center, computed.size(), viewport, Vec2::ZERO);
            apply_drag_delta(&mut transform, delta);
        }
        drag_state.active_panel = None;
        drag_state.last_cursor = None;
        return;
    }

    let Some(cursor) = windows.single().ok().and_then(Window::cursor_position) else {
        return;
    };

    if drag_state.active_panel.is_none() {
        let Some((_, handle)) = handles.iter().find(|(interaction, _)| {
            **interaction == Interaction::Pressed
                || (mouse.just_pressed(MouseButton::Left) && **interaction == Interaction::Hovered)
        }) else {
            return;
        };
        let Ok((_, mut z_index, _, _)) = panels.get_mut(handle.0) else {
            return;
        };
        drag_state.frontmost_z = drag_state.frontmost_z.saturating_add(1);
        z_index.0 = drag_state.frontmost_z;
        drag_state.active_panel = Some(handle.0);
        drag_state.last_cursor = Some(cursor);
        return;
    }

    let Some(last_cursor) = drag_state.last_cursor.replace(cursor) else {
        return;
    };
    let delta = cursor - last_cursor;
    if delta == Vec2::ZERO {
        return;
    }

    let Some(panel) = drag_state.active_panel else {
        return;
    };
    let Ok((mut transform, _, computed, global)) = panels.get_mut(panel) else {
        drag_state.active_panel = None;
        drag_state.last_cursor = None;
        return;
    };
    let (_, _, current_center) = global.to_scale_angle_translation();
    let delta = clamp_drag_delta(current_center, computed.size(), viewport, delta);
    apply_drag_delta(&mut transform, delta);
}

fn clamp_drag_delta(
    current_center: Vec2,
    panel_size: Vec2,
    viewport_size: Vec2,
    delta: Vec2,
) -> Vec2 {
    const SCREEN_MARGIN: f32 = 16.0;
    let half_size = panel_size * 0.5;
    let min_center = half_size + Vec2::splat(SCREEN_MARGIN);
    let max_center = (viewport_size - half_size - Vec2::splat(SCREEN_MARGIN)).max(min_center);
    let desired = (current_center + delta).clamp(min_center, max_center);
    desired - current_center
}

fn apply_drag_delta(transform: &mut UiTransform, delta: Vec2) {
    transform.translation.x = Val::Px(val_as_px(transform.translation.x) + delta.x);
    transform.translation.y = Val::Px(val_as_px(transform.translation.y) + delta.y);
}

fn val_as_px(value: Val) -> f32 {
    match value {
        Val::Px(value) => value,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_drag_delta, clamp_drag_delta, val_as_px};
    use bevy::prelude::{UiTransform, Val, Val2, Vec2};

    #[test]
    fn non_pixel_offsets_start_from_zero() {
        assert_eq!(val_as_px(Val::Auto), 0.0);
        assert_eq!(val_as_px(Val::Percent(25.0)), 0.0);
    }

    #[test]
    fn pixel_offsets_are_preserved_for_incremental_dragging() {
        let mut transform = UiTransform::from_translation(Val2::px(18.0, 32.0));

        apply_drag_delta(&mut transform, Vec2::new(7.5, -4.0));
        apply_drag_delta(&mut transform, Vec2::new(-2.5, 6.0));

        assert_eq!(transform.translation.x, Val::Px(23.0));
        assert_eq!(transform.translation.y, Val::Px(34.0));
    }

    #[test]
    fn drag_delta_keeps_panel_inside_viewport() {
        let delta = clamp_drag_delta(
            Vec2::new(110.0, 100.0),
            Vec2::new(180.0, 120.0),
            Vec2::new(800.0, 600.0),
            Vec2::new(-500.0, -500.0),
        );

        assert_eq!(delta, Vec2::new(-4.0, -24.0));
    }
}
