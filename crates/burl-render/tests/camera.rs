//! Camera matrices against hand-derived GL results (default Gource camera: at (0,0,-300) looking
//! at the origin, fov 90, up = -y).

use burl_render::Camera;
use glam::Vec3;

fn cam() -> Camera {
    Camera {
        pos: Vec3::new(0.0, 0.0, -300.0),
        target: Vec3::ZERO,
        max_distance: 10000.0,
    }
}

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "{a} != {b}");
}

#[test]
fn target_is_the_screen_centre() {
    let p = cam().project(Vec3::ZERO, 640.0, 480.0);
    near(p.x, 320.0);
    near(p.y, 240.0);
}

#[test]
fn world_x_is_right_and_world_y_is_down() {
    // fov 90 at distance 300: half the height is 300 world units, half the width 300 * aspect.
    let c = cam();
    let p = c.project(Vec3::new(100.0, 0.0, 0.0), 640.0, 480.0);
    near(p.x, 320.0 + 100.0 / (300.0 * 640.0 / 480.0) * 320.0);
    near(p.y, 240.0);
    let p = c.project(Vec3::new(0.0, 150.0, 0.0), 640.0, 480.0);
    near(p.x, 320.0);
    near(p.y, 240.0 + 150.0 / 300.0 * 240.0);
}

#[test]
fn nearer_points_have_smaller_depth_and_zfar_follows_max_distance() {
    let c = cam();
    let near_z = c.project(Vec3::new(0.0, 0.0, -100.0), 640.0, 480.0).z;
    let far_z = c.project(Vec3::ZERO, 640.0, 480.0).z;
    assert!(near_z < far_z && (0.0..1.0).contains(&far_z));
    // gluPerspective: m[2][2] = (zf + zn) / (zn - zf) with zf = max_distance + 1.
    let m = c.projection(640.0, 480.0);
    near(m.z_axis.z, (10001.0 + 0.1) / (0.1 - 10001.0));
}

#[test]
fn world_matrix_maps_the_view_frustum_edge_to_ndc() {
    // The top edge of the view at the target plane is (0, -300, 0) (up is -y).
    let n = cam()
        .view_projection(640.0, 480.0)
        .project_point3(Vec3::new(0.0, -300.0, 0.0));
    near(n.y, 1.0);
}
