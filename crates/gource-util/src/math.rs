//! `normalise` from `src/core/vectors.cpp:7-23`: divide by the f32 length, return the
//! zero vector unchanged (glm's `normalize` would give NaN).

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_vector_unchanged() {
        assert_eq!(normalise3([0.0, 0.0, 0.0]), [0.0, 0.0, 0.0]);
        assert_eq!(normalise2([0.0, 0.0]), [0.0, 0.0]);
        assert_eq!(normalise3([0.0, 0.0, 2.0]), [0.0, 0.0, 1.0]);
    }
}
