extern crate cgmath;
extern crate rand;

#[macro_use]
extern crate pixelport_document;

pub mod ordered_float;
pub use ordered_float::*;

use cgmath::*;
use pixelport_document::*;
use std::f32;

use std::hash::Hasher;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32
}
impl Hash for Rectangle {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        let str = format!("{:?}", self);
        str.hash(state);
    }
}

// Standard invert (at the time of writing) doesn't allow small determinants, see https://github.com/bjz/cgmath-rs/issues/210
pub fn mat4_invert(mat: &Matrix4<f32>) -> Matrix4<f32> {
    let det = mat.determinant();
    let one: f32 = one();
    let inv_det = one / det;
    let t = mat.transpose();
    let cf = |i, j| {
        let mat = match i {
            0 => Matrix3::from_cols(t.y.truncate_n(j),
                                    t.z.truncate_n(j),
                                    t.w.truncate_n(j)),
            1 => Matrix3::from_cols(t.x.truncate_n(j),
                                    t.z.truncate_n(j),
                                    t.w.truncate_n(j)),
            2 => Matrix3::from_cols(t.x.truncate_n(j),
                                    t.y.truncate_n(j),
                                    t.w.truncate_n(j)),
            3 => Matrix3::from_cols(t.x.truncate_n(j),
                                    t.y.truncate_n(j),
                                    t.z.truncate_n(j)),
            _ => panic!("out of range")
        };
        let sign = if (i+j) & 1 == 1 {-one} else {one};
        mat.determinant() * sign * inv_det
    };

    Matrix4::new(cf(0, 0), cf(0, 1), cf(0, 2), cf(0, 3),
                 cf(1, 0), cf(1, 1), cf(1, 2), cf(1, 3),
                 cf(2, 0), cf(2, 1), cf(2, 2), cf(2, 3),
                 cf(3, 0), cf(3, 1), cf(3, 2), cf(3, 3))
}




fn hex_to_color(val: &str) -> Option<Vector4<f32>> {
    let mut vals = vec![];
    for i in 0..val.len()/2 {
        let x = match u32::from_str_radix(&val[i*2..i*2+2], 16) {
            Ok(x) => {
                x as f32 / 255.0
            },
            Err(_) => return None
        };
        vals.push(x);
    }
    if vals.len() == 3 {
        Some(Vector4::new(vals[0], vals[1], vals[2], 1.0))
    } else if vals.len() == 4 {
        Some(Vector4::new(vals[1], vals[2], vals[3], vals[0]))
    } else {
        None
    }
}

#[test]
fn test_color_from_hex() {
    assert_eq!(hex_to_color("ff00ff"), Some(Vector4::new(1.0, 0.0, 1.0, 1.0)));
    assert_eq!(hex_to_color("3b0100aa"), Some(Vector4::new(1.0/255.0, 0.0, 170.0/255.0, 59.0/255.0)));
}


pub struct DecomposedCamera {
    pub camera: Matrix4<f32>,
    pub camera_inverse: Matrix4<f32>,
    pub position: Vector3<f32>,
    pub near_point: Vector3<f32>,
    pub far_point: Vector3<f32>,
    pub near: f32,
    pub far: f32,
    pub direction: Vector3<f32>
}

impl DecomposedCamera {
    pub fn from_matrix4(camera: &Matrix4<f32>) -> DecomposedCamera {
        let camera_inverse = mat4_invert(camera);
        let camera_position = camera_inverse * Vector4::new(0.0, 0.0, -1.0, 0.0);
        let camera_position = camera_position.truncate() / camera_position.w;
        let camera_near = camera_inverse * Vector4::new(0.0, 0.0, -1.0, 1.0);
        let camera_near = camera_near.truncate() / camera_near.w;
        let camera_far = camera_inverse * Vector4::new(0.0, 0.0, 1.0, 1.0);
        let camera_far = camera_far.truncate() / camera_far.w;
        DecomposedCamera {
            camera: camera.clone(),
            camera_inverse: camera_inverse,
            position: camera_position,
            near_point: camera_near,
            far_point: camera_far,
            near: (camera_near - camera_position).length(),
            far: (camera_far - camera_position).length(),
            direction: (camera_far - camera_near).normalize()
        }
    }
}

pub fn construct_shadow_camera(decomposed_camera: &DecomposedCamera, cascade_index: i32, n_cascades: i32, shadow_view: &Matrix4<f32>) -> Matrix4<f32> {
    let ratio = decomposed_camera.far / decomposed_camera.near;
    let p0 = (cascade_index as f32) / (n_cascades as f32);
    let p1 = (cascade_index as f32 + 1.0) / (n_cascades as f32);
    let z0 = decomposed_camera.near * ratio.powf(p0);
    let z1 = decomposed_camera.near * ratio.powf(p1);
    let z0 = decomposed_camera.camera * ((decomposed_camera.position + decomposed_camera.direction * z0).extend(1.0));
    let z1 = decomposed_camera.camera * ((decomposed_camera.position + decomposed_camera.direction * z1).extend(1.0));
    let z0 = z0.z / z0.w;
    let z1 = z1.z / z1.w;
    let mut frustum = vec![
        decomposed_camera.camera_inverse * Vector4::new(-1.0, -1.0, z0, 1.0),
        decomposed_camera.camera_inverse * Vector4::new(-1.0,  1.0, z0, 1.0),
        decomposed_camera.camera_inverse * Vector4::new( 1.0, -1.0, z0, 1.0),
        decomposed_camera.camera_inverse * Vector4::new( 1.0,  1.0, z0, 1.0),
        decomposed_camera.camera_inverse * Vector4::new(-1.0, -1.0, z1, 1.0),
        decomposed_camera.camera_inverse * Vector4::new(-1.0,  1.0, z1, 1.0),
        decomposed_camera.camera_inverse * Vector4::new( 1.0, -1.0, z1, 1.0),
        decomposed_camera.camera_inverse * Vector4::new( 1.0,  1.0, z1, 1.0),
    ];
    for l in 0..frustum.len() {
        frustum[l] = frustum[l] / frustum[l].w;
    }
    // move to shadow cam space
    for l in 0..frustum.len() {
        frustum[l] = shadow_view * frustum[l];
        frustum[l] = frustum[l] / frustum[l].w;
    }
    // find min and max in shadow space
    let mut min = frustum[0].clone();
    let mut max = frustum[0].clone();
    for l in 1..frustum.len() {
        min.x = min.x.min(frustum[l].x);
        min.y = min.y.min(frustum[l].y);
        min.z = min.z.min(frustum[l].z);
        max.x = max.x.max(frustum[l].x);
        max.y = max.y.max(frustum[l].y);
        max.z = max.z.max(frustum[l].z);
    }
    return &ortho(min.x, max.x, min.y, max.y, -max.z, -min.z) * shadow_view;
}


pub fn pon_std(translater: &mut PonTranslater) {
    pon_register_functions!("Standard Library", translater =>

        "Generate random float",
        random_float() f32 => {
            Ok(rand::random::<f32>())
        }

        "Divide two numbers",
        div(vals: [f32]) f32 => {
            Ok(vals[0] / vals[1])
        }

        "Add two numbers",
        add(vals: [f32]) f32 => {
            Ok(vals.iter().fold(0.0, |acc, &v| acc + v))
        }

        "Multiply a list of numbers",
        mul(vals: [f32]) f32 => {
            Ok(vals.iter().fold(1.0, |acc, &v| acc * v))
        }

        "Negate a number",
        neg(val: (f32)) f32 => {
            Ok(-val)
        }

        "Pi.",
        pi() f32 => {
            Ok(std::f32::consts::PI)
        }

        "Absolute value of a number",
        abs(val: (f32)) f32 => {
            Ok(val.abs())
        }

        "Minimum value of a list of numbers",
        min(vals: [f32]) f32 => {
            Ok(vals.iter().cloned().fold(std::f32::MAX, f32::min))
        }

        "Shorthand for creating a vector4",
        color({
            r: (f32) | 0.0,
            g: (f32) | 0.0,
            b: (f32) | 0.0,
            a: (f32) | 1.0,
        }) Vector4<f32> => {
            Ok(Vector4::new(r, g, b, a))
        }

        "Shorthand for creating a vector4 from a hex string",
        color_from_hex(val: (String)) Vector4<f32> => {
            match hex_to_color(&val) {
                Some(color) => Ok(color),
                None => Err(PonTranslaterErr::Generic(format!("Cannot parse color value: {}", val)))
            }
        }

        "Convert a number to a string",
        num_to_string(val: (f32)) String => {
            Ok(val.to_string())
        }

        "Concatenate a list of strings",
        string_concat(vals: [String]) String => {
            Ok(vals.iter().fold("".to_string(), |acc, v| acc + &v))
        }

        "Compare two strings",
        string_compare(vals: [String]) bool => {
            Ok(vals[0] == vals[1])
        }

        "Return either a or b depending on test",
        switch_number({
            test: (bool),
            a: (f32),
            b: (f32),
        }) f32 => {
            if test {
                Ok(a)
            } else {
                Ok(b)
            }
        }

        "Not boolean operator",
        not(v: (bool)) bool => {
            Ok(!v)
        }

        "And boolean operator",
        and(vals: [bool]) bool => {
            Ok(vals.iter().fold(true, |acc, &v| acc && v))
        }

        "Or boolean operator",
        or(vals: [bool]) bool => {
            Ok(vals.iter().fold(false, |acc, &v| acc || v))
        }

        "Create a vec2",
        vec2({
            x: (f32) | 0.0,
            y: (f32) | 0.0,
        }) Vector2<f32> => {
            Ok(Vector2::new(x, y))
        }

        "Create a vec3",
        vec3({
            x: (f32) | 0.0,
            y: (f32) | 0.0,
            z: (f32) | 0.0,
        }) Vector3<f32> => {
            Ok(Vector3::new(x, y, z))
        }

        "Create a vec4",
        vec4({
            x: (f32) | 0.0,
            y: (f32) | 0.0,
            z: (f32) | 0.0,
            w: (f32) | 0.0,
        }) Vector4<f32> => {
            Ok(Vector4::new(x, y, z, w))
        }

        "Creata a vec3 from spherical coordinates",
        spherical_vec3({
            r: (f32) | 0.0,
            theta: (f32) | 0.0,
            phi: (f32) | 0.0,
        }) Vector3<f32> => {
            Ok(Vector3::new(r * theta.sin() * phi.cos(),
                            r * theta.sin() * phi.sin(),
                            r * theta.cos()))
        }

        "Add two vec3",
        add3(vals: [Vector3<f32>]) Vector3<f32> => {
            Ok(&vals[0] + &vals[1])
        }

        "Subtract two vec3",
        sub3(vals: [Vector3<f32>]) Vector3<f32> => {
            Ok(&vals[0] - &vals[1])
        }

        "Normalize a vec3",
        normalize3(vec: (Vector3<f32>)) Vector3<f32> => {
            Ok(vec.normalize())
        }

        "Snap a vec3 to a set grid",
        snap3({
            vec: (Vector3<f32>),
            snap: (f32),
        }) Vector3<f32> => {
            Ok(Vector3::new((vec.x / snap).floor() * snap, (vec.y / snap).floor() * snap, (vec.z / snap).floor() * snap))
        }

        "Create a matrix from values",
        matrix(data : [f32]) Matrix4<f32> => {
            Ok(Matrix4::new(
                data[0], data[1], data[2], data[3],
                data[4], data[5], data[6], data[7],
                data[8], data[9], data[10], data[11],
                data[12], data[13], data[14], data[15]))
        }

        "Create a list of matrices",
        matrices(data : [Matrix4<f32>]) Vec<Matrix4<f32>> => {
            Ok(data)
        }

        "Generate a matrix from a translation",
        translate(vec3 : (Vector3<f32>)) Matrix4<f32> => {
            Ok(Matrix4::from_translation(vec3))
        }

        "Generate a matrix from a rotation around x.",
        rotate_x(v : (f32)) Matrix4<f32> => {
            Ok(Quaternion::from_angle_x(Rad { s: v }).into())
        }

        "Generate a matrix from a rotation around y.",
        rotate_y(v : (f32)) Matrix4<f32> => {
            Ok(Quaternion::from_angle_y(Rad { s: v }).into())
        }

        "Generate a matrix from a rotation around z.",
        rotate_z(v : (f32)) Matrix4<f32> => {
            Ok(Quaternion::from_angle_z(Rad { s: v }).into())
        }

        "Generate a matrix from a quaternion rotation.",
        rotate_quaternion(v : (Vector4<f32>)) Matrix4<f32> => {
            Ok(Quaternion::new(v.w, v.x, v.y, v.z).into())
        }

        "Generate a matrix from a scaling vector",
        scale(v : (Vector3<f32>)) Matrix4<f32> => {
            Ok(Matrix4::new(
                v.x,  zero(), zero(), zero(),
                zero(), v.y,  zero(), zero(),
                zero(), zero(), v.z,  zero(),
                zero(), zero(), zero(), one()))
        }

        "Generate a lookat matrix",
        lookat({
            eye: (Vector3<f32>),
            center: (Vector3<f32>),
            up: (Vector3<f32>) | Vector3::new(0.0, 0.0, 1.0),
        }) Matrix4<f32> => {
            Ok(Matrix4::look_at(Point3::from_vec(eye), Point3::from_vec(center), up))
        }

        "Generate a projection matrix.",
        projection({
            fovy: (f32) | 1.0,
            aspect: (f32) | 1.0,
            near: (f32) | 0.1,
            far: (f32) | 10.0,
        }) Matrix4<f32> => {
            Ok(perspective(Rad { s: fovy }, aspect, near, far))
        }

        "Generate an ortographic matrix.",
        orthographic({
            left: (f32) | 0.0,
            right: (f32) | 0.0,
            top: (f32) | 0.0,
            bottom: (f32) | 0.0,
            near: (f32) | 0.0,
            far: (f32) | 0.0,
        }) Matrix4<f32> => {
            if left - right == 0.0 || bottom - top == 0.0 || near - far == 0.0 {
                Err(PonTranslaterErr::Generic(
                    format!("Orthographic area is zero, left: {}, right: {}, top: {}, bottom: {}, near: {}, far: {}", left, right, top, bottom, near, far)))
            } else {
                Ok(ortho(left, right, bottom, top, near, far))
            }
        }

        "Create a shadow map camera from a regular camera.",
        shadow_camera({
            camera: (Matrix4<f32>),
            light_direction: (Vector3<f32>),
            cascade_index: (f32) | 0.0,
            n_cascades: (f32) | 1.0,
        }) Matrix4<f32> => {
            let shadow_view = Matrix4::look_at(Point3::from_vec(light_direction), Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 1.0));
            Ok(construct_shadow_camera(&DecomposedCamera::from_matrix4(&camera), cascade_index as i32, n_cascades as i32, &shadow_view))
        }


        "Multiply a number of matrices together",
        matrix_mul(data : [Matrix4<f32>]) Matrix4<f32> => {
            let mut a: Matrix4<f32> = Matrix4::identity();
            for b in data {
                a = &a * &b;
            }
            Ok(a)
        }

        "Invert a matrix",
        invert(mat: (Matrix4<f32>)) Matrix4<f32> => {
            Ok(mat4_invert(&mat))
        }

        "Generate an identity matrix",
        identity() Matrix4<f32> => {
            Ok(Matrix4::identity())
        }

        "Generate a billboard matrix",
        billboard({
            view: (Matrix4<f32>),
            translate: (Vector3<f32>) | Vector3::zero(),
        }) Matrix4<f32> => {
            Ok(Matrix4::new(
                view.x.x, view.y.x, view.z.x, zero(),
                view.x.y, view.y.y, view.z.y, zero(),
                view.x.z, view.y.z, view.z.z, zero(),
                translate.x, translate.y, translate.z, one()))
        }

        "Creata a rectangle",
        rectangle({
            x: (f32) | 0.0,
            y: (f32) | 0.0,
            width: (f32),
            height: (f32),
        }) Rectangle => {
            Ok(Rectangle { x: x, y: y, width: width, height: height })
        }

    );
}

#[test]
fn test_translate_vec3() {
    let mut translater = PonTranslater::new();
    pon_std(&mut translater);
    let doc = Document::new(translater);
    let pon = Pon::from_string("vec3 { x: 1, y: 2, z: 0 }").unwrap();
    assert_eq!(doc.translater.translate::<Vector3<f32>>(&pon, &doc.bus), Ok(Vector3::new(1.0, 2.0, 0.0)))
}

#[test]
fn test_translate_vec3_defaults() {
    let mut translater = PonTranslater::new();
    pon_std(&mut translater);
    let doc = Document::new(translater);
    let pon = Pon::from_string("vec3 { x: 1, z: 0 }").unwrap();
    assert_eq!(doc.translater.translate::<Vector3<f32>>(&pon, &doc.bus), Ok(Vector3::new(1.0, 0.0, 0.0)))
}

#[test]
fn test_translate_matrix() {
    let mut translater = PonTranslater::new();
    pon_std(&mut translater);
    let doc = Document::new(translater);
    let pon = Pon::from_string("matrix [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]").unwrap();
    assert_eq!(doc.translater.translate::<Matrix4<f32>>(&pon, &doc.bus), Ok(Matrix4::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0)))
}

#[test]
fn test_translate_matrix_translate() {
    let mut translater = PonTranslater::new();
    pon_std(&mut translater);
    let doc = Document::new(translater);
    let pon = Pon::from_string("translate vec3 { x: 1.0 }").unwrap();
    assert_eq!(doc.translater.translate::<Matrix4<f32>>(&pon, &doc.bus),
        Ok(Matrix4::from_translation(Vector3::new(1.0, 0.0, 0.0))));
}

#[test]
fn test_translate_matrix_lookat() {
    let mut translater = PonTranslater::new();
    pon_std(&mut translater);
    let doc = Document::new(translater);
    let pon = Pon::from_string("lookat { eye: vec3 { x: 1 }, center: vec3 { y: -2 } }").unwrap();
    assert_eq!(doc.translater.translate::<Matrix4<f32>>(&pon, &doc.bus),
        Ok(Matrix4::look_at(Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, -2.0, 0.0), Vector3::new(0.0, 0.0, 1.0))));
}

#[test]
fn test_translate_matrix_mul() {
    let mut translater = PonTranslater::new();
    pon_std(&mut translater);
    let doc = Document::new(translater);
    let pon = Pon::from_string("matrix_mul [translate vec3 { x: 1.0 }, translate vec3 { x: 1.0, y: -4.0 } ]").unwrap();
    assert_eq!(doc.translater.translate::<Matrix4<f32>>(&pon, &doc.bus),
        Ok(Matrix4::from_translation(Vector3::new(2.0, -4.0, 0.0))));
}
