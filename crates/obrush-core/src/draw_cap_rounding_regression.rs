//! Independent f32 cap-rounding fixture. Included from brush.rs under cfg(test).
use super::*;

#[test]
fn mirrored_one_ulp_cap_repair_keeps_original_guard_and_ray() {
    let mut old = vec![
        Vec3::new(-0.1, -0.1, 0.0),
        Vec3::new(0.1, -0.1, 0.0),
        Vec3::new(0.0, 0.1, 0.0),
        Vec3::new(0.25, 0.125, 0.0),
        Vec3::ZERO,
    ];
    old[4] = reflect_point(old[3], 0.0);
    let face_cross =
        (old[1].as_dvec3() - old[0].as_dvec3()).cross(old[2].as_dvec3() - old[0].as_dvec3());
    let guard = DeformationGuard {
        faces: vec![FaceGuard {
            indices: [0, 1, 2],
            cross: face_cross,
            minimum_projection: face_cross.length_squared() * 0.5,
            minimum_quality: 0.05,
        }],
        folds: vec![],
    };
    let anchor = DrawContactAnchor {
        ids: [0, 1, 2],
        bary: [0.25, 0.25, 0.5],
    };
    let x_one = old[3].x.next_up();
    let x_two = x_one.next_up();
    let x_three = x_two.next_up();
    let y_three = old[3].y.next_up().next_up().next_up();
    let mut proposed = old.clone();
    let mut candidate = old.clone();
    for id in 0..3 {
        proposed[id].z += 0.002;
        candidate[id].z += 0.001;
    }
    proposed[3] = Vec3::new(x_two, y_three, 0.0);
    proposed[4] = reflect_point(proposed[3], 0.0);
    candidate[3] = Vec3::new(x_three, old[3].y, 0.0);
    candidate[4] = reflect_point(candidate[3], 0.0);
    let allowed = (proposed[3].as_dvec3() - old[3].as_dvec3()).length();
    let before = (candidate[3].as_dvec3() - old[3].as_dvec3()).length();
    assert!(before > allowed, "control must exceed authored cap");
    assert!(guard.safe(&candidate));

    repair_draw_rounded_caps(
        &mut candidate,
        &old,
        &proposed,
        &[0, 1, 2, 3],
        Some(&[None, None, None, Some(4)]),
        0.0,
        &guard,
        anchor,
        Vec3::ZERO,
        Vec3::Z,
        0.18,
    );
    assert_eq!(candidate[3].x, x_two, "only one inward f32 step is needed");
    assert_eq!(candidate[4], reflect_point(candidate[3], 0.0));
    assert!(guard.safe(&candidate));
    for &id in &[0, 1, 2, 3, 4] {
        let realized = (candidate[id].as_dvec3() - old[id].as_dvec3()).length();
        let authored = (proposed[id].as_dvec3() - old[id].as_dvec3()).length();
        assert!(realized <= authored, "cap exceeded at {id}");
    }
    let lift = draw_local_ray_lift(&candidate, &guard, anchor, Vec3::ZERO, Vec3::Z, 0.18).unwrap()
        - draw_local_ray_lift(&old, &guard, anchor, Vec3::ZERO, Vec3::Z, 0.18).unwrap();
    assert!(lift >= 0.00018);
}
