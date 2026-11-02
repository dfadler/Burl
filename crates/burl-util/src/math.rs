//! `normalise` from `src/core/vectors.cpp:7-23`: divide by the f32 length, return the
//! zero vector unchanged (glm's `normalize` would give NaN).

/// `src/core/pi.h:31-33`: the C++ macros are f64 literals with these exact (truncated) values.
#[allow(clippy::approx_constant)] // the truncated C++ value is the point
pub const PI: f64 = 3.14159265;
pub const RADIANS_TO_DEGREES: f64 = 57.29577951;
pub const DEGREES_TO_RADIANS: f64 = 0.017453292;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];

pub fn normalise2(v: Vec2) -> Vec2 {
    let l = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if l > 0.0 {
        [v[0] / l, v[1] / l]
    } else {
        v
    }
}

pub fn normalise3(v: Vec3) -> Vec3 {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 0.0 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        v
    }
}

/// `normalise(vec2)` on glam types; same zero-vector rule as [`normalise2`].
pub fn normalise_vec2(v: glam::Vec2) -> glam::Vec2 {
    normalise2(v.to_array()).into()
}

/// `rotate_vec2` from `src/core/vectors.cpp:3-5`: `s`/`c` are the caller's `sinf`/`cosf`.
pub fn rotate_vec2(v: glam::Vec2, s: f32, c: f32) -> glam::Vec2 {
    glam::vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_vector_unchanged() {
        assert_eq!(normalise3([0.0, 0.0, 0.0]), [0.0, 0.0, 0.0]);
        assert_eq!(normalise2([0.0, 0.0]), [0.0, 0.0]);
        assert_eq!(normalise3([0.0, 0.0, 2.0]), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn rotate_quarter_turn_and_formula() {
        let r = rotate_vec2(glam::vec2(1.0, 0.0), 1.0, 0.0);
        assert_eq!(r, glam::vec2(0.0, 1.0));
        // sign convention of the C++ formula, not a generic rotation: (2,3) with s=0.5, c=0.25
        assert_eq!(
            rotate_vec2(glam::vec2(2.0, 3.0), 0.5, 0.25),
            glam::vec2(2.0 * 0.25 - 3.0 * 0.5, 2.0 * 0.5 + 3.0 * 0.25)
        );
    }

    #[test]
    fn pi_macros_keep_their_truncated_values() {
        #[allow(clippy::approx_constant)]
        let want = 3.14159265;
        assert_eq!(PI, want);
        assert_eq!(DEGREES_TO_RADIANS as f32, 0.017453292_f32);
        assert_ne!(DEGREES_TO_RADIANS, std::f64::consts::PI / 180.0);
    }
    #[test]
    fn normalise_vec2_matches_array_version() {
        assert_eq!(normalise_vec2(glam::vec2(3.0, 4.0)), glam::vec2(0.6, 0.8));
        assert_eq!(normalise_vec2(glam::Vec2::ZERO), glam::Vec2::ZERO);
    }
}
