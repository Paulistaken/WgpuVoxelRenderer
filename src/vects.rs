#[derive(Default,Debug,Clone,Copy, PartialEq)]
pub struct Vec4f{
    data : wide::f32x4,
}
impl From<f32> for Vec4f{
    fn from(value: f32) -> Self {
        Vec4f { data: wide::f32x4::splat(value) }
    }
}
impl From<[f32;3]> for Vec4f{
    fn from(value: [f32;3]) -> Self {
        Vec4f { data: wide::f32x4::from([value[0],value[1],value[2],0.]) }
    }
}
impl From<[f32;4]> for Vec4f{
    fn from(value: [f32;4]) -> Self {
        Vec4f { data: wide::f32x4::from(value) }
    }
}
impl Vec4f{
    pub const ZERO : Self = Self{data:wide::f32x4::ZERO};
    pub fn to_array(self) -> [f32;4]{
        self.data.to_array()
    }
    pub fn x(&self) -> f32{
        self.data.to_array()[0]
    }
    pub fn y(&self) -> f32{
        self.data.to_array()[1]
    }
    pub fn z(&self) -> f32{
        self.data.to_array()[2]
    }
    pub fn w(&self) -> f32{
        self.data.to_array()[3]
    }
    pub fn lengh(&self) -> f32{
        (self.x().powi(2)+self.y().powi(2)+self.z().powi(2)+self.w().powi(2)).sqrt()
    }
}
impl std::ops::Add for Vec4f{
    type Output = Vec4f;
    fn add(self, rhs: Self) -> Self::Output {
        Vec4f{data:self.data+rhs.data}
    }
}
impl std::ops::AddAssign for Vec4f{
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl std::ops::Sub for Vec4f{
    type Output = Vec4f;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec4f{data:self.data-rhs.data}
    }
}
impl std::ops::Mul for Vec4f{
    type Output = Vec4f;
    fn mul(self, rhs: Self) -> Self::Output {
        Vec4f{data:self.data*rhs.data}
    }
}
impl std::ops::Div for Vec4f{
    type Output = Vec4f;
    fn div(self, rhs: Self) -> Self::Output {
        Vec4f{data:self.data / rhs.data}
    }
}
