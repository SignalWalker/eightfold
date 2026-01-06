use nalgebra::{
    point, vector, Isometry3, Matrix4, Perspective3, Point3, Projective3, Unit, UnitComplex,
    UnitQuaternion, UnitVector3, Vector2, Vector3,
};

mod turn;
pub use turn::*;

mod polar;
pub use polar::*;

#[cfg(test)]
mod test;

pub struct FlyCamera {
    pub position: Point3<f32>,
    pub rotation: UnitQuaternion<f32>,

    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

//impl FlyCamera {
//    pub fn proj_view_mat(&self, aspect: f32) -> Matrix4<f32> {
//        let
//    }
//}

#[derive(Default, Debug, Clone, Copy)]
pub struct OrbitCamera {
    pub target: Point3<f32>,
    offset: Polar3,

    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl OrbitCamera {
    pub const fn new(
        target: Point3<f32>,
        offset: Polar3,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            target,
            offset,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn orbit(&mut self, amt: Turn) {
        self.offset.azi += amt;
    }

    pub fn incline(&mut self, amt: Turn) {
        self.offset.inc += amt;
    }

    pub fn dolly(&mut self, amt: f32) {
        self.offset.rad += amt;
    }

    /// Normal vector pointing from `offset` to `target`.
    pub fn forward(&self) -> Unit<Vector3<f32>> {
        -self.offset.into_cartesian_normal()
    }

    // Move the target along the XZ plane.
    pub fn translate(&mut self, xz: Vector2<f32>) {
        let amt = self.offset.azi.inverse().unit_complex32() * xz;
        self.target.x += amt.x;
        self.target.z += amt.y;
    }

    pub fn distance(&self) -> f32 {
        self.offset.rad
    }

    /// The world position of this camera.
    pub fn position(&self) -> Point3<f32> {
        self.target + self.offset.into_cartesian_vector()
    }

    pub fn look_at(&self) -> Isometry3<f32> {
        Isometry3::look_at_lh(&self.position(), &self.target, &Vector3::new(0.0, 1.0, 0.0))
    }

    /// Generate a perspective projection matrix from the point of view of this camera.
    pub fn proj_view_mat(&self, aspect: f32) -> Matrix4<f32> {
        let look_at = self.look_at();
        let proj = nalgebra_glm::perspective_lh_zo(aspect, self.fovy, self.znear, self.zfar);
        proj * look_at.to_homogeneous()
    }
}
