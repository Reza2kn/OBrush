use super::*;
use serde::ser::{SerializeSeq, Serializer};
use std::collections::BTreeMap;
fn encode<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {
    let mut out = Vec::new();
    to_writer(&mut out, value).unwrap();
    out
}
fn word32(bytes: &[u8]) -> u32 {
    match bytes[0] {
        0x80..=0x8f => {
            assert_eq!(bytes.len(), 1);
            COMMON_F32[(bytes[0] - 0x80) as usize].to_bits()
        }
        0xa0..=0xa4 => {
            let skip = (bytes[0] - 0xa0) as usize;
            assert_eq!(bytes.len(), 5 - skip);
            let mut word = [0; 4];
            word[skip..].copy_from_slice(&bytes[1..]);
            u32::from_le_bytes(word)
        }
        other => panic!("unexpected f32 tag {other}"),
    }
}
fn word64(bytes: &[u8]) -> u64 {
    match bytes[0] {
        0x90..=0x9f => {
            assert_eq!(bytes.len(), 1);
            (COMMON_F32[(bytes[0] - 0x90) as usize] as f64).to_bits()
        }
        0xb0..=0xb4 => {
            let skip = (bytes[0] - 0xb0) as usize;
            assert_eq!(bytes.len(), 5 - skip);
            let mut b = [0; 4];
            b[skip..].copy_from_slice(&bytes[1..]);
            (f32::from_bits(u32::from_le_bytes(b)) as f64).to_bits()
        }
        0xc0 => {
            assert_eq!(bytes.len(), 9);
            u64::from_le_bytes(bytes[1..].try_into().unwrap())
        }
        0xd1..=0xd7 => {
            assert_eq!(bytes.len(), 1 + (bytes[0] - 0xd0) as usize);
            std::str::from_utf8(&bytes[1..])
                .unwrap()
                .parse::<f64>()
                .unwrap()
                .to_bits()
        }
        other => panic!("unexpected f64 tag {other}"),
    }
}
#[test]
fn b01_literal_golden_scalars_strings_arrays_and_names_are_unambiguous() {
    assert_eq!(encode(&false), [1]);
    assert_eq!(encode(&true), [2]);
    assert_eq!(encode(&-1_i32), [5, 1]);
    assert_eq!(encode(&128_u32), [10, 128, 1]);
    assert_eq!(encode(&0_f32), [0x80]);
    assert_eq!(encode(&-0_f32), [0x8b]);
    assert_eq!(encode(&'💙'), [15, 0x99, 0xe9, 7]);
    assert_eq!(
        encode("é\0💙"),
        [16, 7, 0xc3, 0xa9, 0, 0xf0, 0x9f, 0x92, 0x99]
    );
    assert_eq!(encode(&[1_u8, 2, 3]), [0x43, 8, 1, 8, 2, 8, 3]);
    #[derive(serde::Serialize)]
    struct Named {
        x: f32,
    }
    assert_eq!(encode(&Named { x: 0.0 }), [0x61, 16, 1, b'x', 0x80]);
    let mut m = BTreeMap::new();
    m.insert("x", 0_f32);
    assert_eq!(encode(&Named { x: 0.0 }), encode(&m));
    let mut minimum = vec![7];
    minimum.extend([255; 18]);
    minimum.push(3);
    assert_eq!(encode(&i128::MIN), minimum);
    minimum[0] = 12;
    assert_eq!(encode(&u128::MAX), minimum);
}
#[test]
fn b02_all_common_and_fixed_bit_word_f32_samples_round_trip_exactly() {
    for (i, v) in COMMON_F32.iter().enumerate() {
        assert_eq!(common_f32(v.to_bits()), Some(i as u8));
        assert_eq!(encode(v), [0x80 + i as u8]);
    }
    let mut bits = 0xd15ea5e5_u32;
    for _ in 0..32768 {
        bits = bits.wrapping_mul(1664525).wrapping_add(1013904223);
        let v = f32::from_bits(bits);
        let b = encode(&v);
        assert_eq!(word32(&b), bits);
        if v.is_finite() {
            assert!(b.len() <= serde_json::to_vec(&v).unwrap().len() + 1);
        }
    }
    for bits in [
        0, 0x80000000, 1, 0x7f7fffff, 0x7f800000, 0xff800000, 0x7f800001, 0x7fc00042,
    ] {
        assert_eq!(word32(&encode(&f32::from_bits(bits))), bits);
    }
}
#[test]
fn b03_f64_short_decimals_exact_promotions_signed_zero_and_nan_bits_survive() {
    for v in [
        0_f64,
        -0.0,
        0.1,
        0.01,
        1e-7,
        1e-9,
        0.75,
        1000000.0,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(1),
        f64::from_bits(0x7ff0000000000001),
        f64::from_bits(0xfff8000000000042),
    ] {
        assert_eq!(word64(&encode(&v)), v.to_bits());
    }
    let mut bits = 0xf00d_badc_1234_5678_u64;
    for _ in 0..8192 {
        bits = bits
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let v = f64::from_bits(bits);
        let b = encode(&v);
        assert_eq!(word64(&b), bits);
        if v.is_finite() {
            assert!(b.len() <= serde_json::to_vec(&v).unwrap().len() + 1);
        }
    }
}
#[test]
fn b04_optional_presence_variants_and_nested_empty_values_do_not_collapse() {
    assert_ne!(encode(&None::<Vec<u8>>), encode(&Some(Vec::<u8>::new())));
    assert_ne!(
        encode(&Vec::<u8>::new()),
        encode(&BTreeMap::<String, u8>::new())
    );
    assert_ne!(encode(&()), encode(&None::<()>));
    #[derive(serde::Serialize)]
    enum V {
        Unit,
        New(u8),
        Pair(u8, u8),
        Record { x: u8 },
    }
    let all = [
        encode(&V::Unit),
        encode(&V::New(1)),
        encode(&V::Pair(1, 2)),
        encode(&V::Record { x: 1 }),
    ];
    for (i, a) in all.iter().enumerate() {
        for b in all.iter().skip(i + 1) {
            assert_ne!(a, b);
        }
    }
    assert_ne!(
        encode(&vec![vec![1_u8, 2], vec![3]]),
        encode(&vec![vec![1_u8], vec![2, 3]])
    );
}
#[test]
fn b05_unknown_sequence_lengths_and_inconsistent_manual_counts_are_explicit() {
    struct Unknown;
    impl Serialize for Unknown {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut seq = s.serialize_seq(None)?;
            seq.serialize_element(&1_u8)?;
            seq.end()
        }
    }
    struct Bad;
    impl Serialize for Bad {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut seq = s.serialize_seq(Some(2))?;
            seq.serialize_element(&1_u8)?;
            seq.end()
        }
    }
    assert_eq!(encode(&Unknown), [0x50, 8, 1, 0xff]);
    assert_ne!(encode(&Unknown), encode(&[1_u8]));
    assert!(to_writer(&mut Vec::new(), &Bad).is_err());
    struct Deep(u8);
    impl Serialize for Deep {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            if self.0 == 0 {
                s.serialize_unit()
            } else {
                s.serialize_some(&Deep(self.0 - 1))
            }
        }
    }
    assert!(to_writer(&mut Vec::new(), &Deep(65)).is_err());
}
#[test]
fn b06_short_writes_errors_and_display_projection_keep_the_caller_bound() {
    struct Limited {
        bytes: Vec<u8>,
        left: usize,
    }
    impl io::Write for Limited {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if self.left == 0 {
                return Err(io::Error::other("literal bound"));
            }
            let n = b.len().min(self.left);
            self.bytes.extend_from_slice(&b[..n]);
            self.left -= n;
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut sink = Limited {
        bytes: Vec::new(),
        left: 3,
    };
    assert!(to_writer(&mut sink, "long literal").is_err());
    assert_eq!(sink.bytes.len(), 3);
    struct DisplayOnly;
    impl fmt::Display for DisplayOnly {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "é\0💙")
        }
    }
    impl Serialize for DisplayOnly {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.collect_str(self)
        }
    }
    assert_eq!(encode(&DisplayOnly), encode("é\0💙"));
    let mut sink = Limited {
        bytes: Vec::new(),
        left: 3,
    };
    assert!(to_writer(&mut sink, &DisplayOnly).is_err());
    assert_eq!(sink.bytes.len(), 3);
}
fn doc() -> super::super::Document {
    let mut m = crate::Mesh::sphere(1);
    m.mask_convention = crate::MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    m.colors.fill([0.125, 0.25, 0.375]);
    m.materials = Some(vec![[0.5, 0.25]; m.positions.len()]);
    super::super::Document::new(m)
}
#[test]
fn b07_same_project_projection_and_each_persistent_mesh_field_change_agree_with_json() {
    use super::super::{binding, binding_matches, original_json_binding};
    let original = doc();
    let limit = crate::MAX_SOURCE_GEOMETRY_OPERATION_BYTES;
    let bin = binding(&original, None, limit).unwrap();
    let json = original_json_binding(&original, None, limit).unwrap();
    assert!(binding_matches(&original, &bin, limit).unwrap());
    type Change = fn(&mut super::super::Document);
    let changes: [Change; 9] = [
        |d| d.name.push('💙'),
        |d| d.mesh.as_mut().unwrap().positions[0].x += 0.03125,
        |d| d.mesh.as_mut().unwrap().normals[0].z = 0.333,
        |d| d.mesh.as_mut().unwrap().masks[0] = 0.5,
        |d| d.mesh.as_mut().unwrap().colors[0][2] = 0.25,
        |d| d.mesh.as_mut().unwrap().materials = None,
        |d| d.mesh.as_mut().unwrap().materials.as_mut().unwrap()[0][0] = 0.75,
        |d| d.mesh.as_mut().unwrap().triangles.swap(0, 1),
        |d| d.mesh.as_mut().unwrap().mask_convention = crate::MaskConvention::ProtectedF32V1,
    ];
    for change in changes {
        let mut changed = doc();
        change(&mut changed);
        assert_ne!(original_json_binding(&changed, None, limit).unwrap(), json);
        assert_ne!(binding(&changed, None, limit).unwrap(), bin);
        assert!(!binding_matches(&changed, &bin, limit).unwrap());
    }
}
#[test]
fn b08_zero_heavy_mesh_presence_and_evidence_allocation_remain_bounded() {
    use super::super::{binding, original_json_binding, BINDING_REFERENCE_SCRATCH_BYTES};
    let mut d = doc();
    let m = d.mesh.as_mut().unwrap();
    m.colors.fill([0.; 3]);
    m.normals.fill(crate::Vec3::Z);
    m.masks.fill(1.);
    m.materials = Some(vec![[0.; 2]; m.positions.len()]);
    let limit = crate::MAX_SOURCE_GEOMETRY_OPERATION_BYTES;
    let b = binding(&d, None, limit).unwrap();
    let j = original_json_binding(&d, None, limit).unwrap();
    assert!(b.len() <= j.len(), "zero-heavy encoding enlarged");
    assert_eq!(
        binding(&d, None, b.len() + BINDING_REFERENCE_SCRATCH_BYTES).unwrap(),
        b
    );
    assert!(binding(&d, None, b.len() + BINDING_REFERENCE_SCRATCH_BYTES - 1).is_err());
    let before = b;
    d.mesh.as_mut().unwrap().materials = None;
    assert_ne!(binding(&d, None, limit).unwrap(), before);
}
#[test]
fn b09_comparison_rejects_truncated_extra_and_mutated_bytes_without_allocating_a_new_binding() {
    use super::super::{binding, binding_matches};
    let d = doc();
    let limit = crate::MAX_SOURCE_GEOMETRY_OPERATION_BYTES;
    let b = binding(&d, None, limit).unwrap();
    assert!(!binding_matches(&d, &b[..b.len() - 1], limit).unwrap());
    let mut extra = b.clone();
    extra.push(255);
    assert!(!binding_matches(&d, &extra, limit).unwrap());
    let mut wrong = b;
    wrong[0] ^= 1;
    assert!(!binding_matches(&d, &wrong, limit).unwrap());
    assert!(binding_matches(&d, &wrong, 0).is_err());
}
#[test]
fn b10_human_readable_serialize_branch_and_float_type_width_are_explicit() {
    struct Branch;
    impl Serialize for Branch {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let human = s.is_human_readable();
            s.serialize_str(if human { "human" } else { "other" })
        }
    }
    assert_eq!(encode(&Branch), encode("human"));
    assert_eq!(serde_json::to_string(&Branch).unwrap(), "\"human\"");
    assert_ne!(encode(&1_f32), encode(&1_f64));
    assert_ne!(encode(&1_u32), encode(&1_u64));
}
