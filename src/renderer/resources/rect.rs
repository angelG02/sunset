use cgmath::{BaseNum, Vector2, Zero};

/// A rectangle defined by two points. There is no defined origin, so 0,0 could be anywhere
/// (top-left, bottom-left, etc)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rect<T: BaseNum> {
    /// The beginning point of the rect
    pub min: cgmath::Vector2<T>,
    /// The ending point of the rect
    pub max: cgmath::Vector2<T>,
}

impl<T: BaseNum> Default for Rect<T> {
    fn default() -> Self {
        Rect {
            min: cgmath::Vector2::<T>::zero(),
            max: cgmath::Vector2::new(T::zero(), T::zero()),
        }
    }
}

impl<T: BaseNum> Rect<T> {
    pub fn width(&self) -> T {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> T {
        self.max.y - self.min.y
    }

    pub fn has_point(&self, point: &Vector2<T>) -> bool {
        if point.x > self.min.x
            && point.x < self.max.x
            && point.y > self.min.y
            && point.y < self.max.y
        {
            return true;
        }
        false
    }
}

unsafe impl<T: BaseNum> bytemuck::Zeroable for Rect<T> {}
unsafe impl<T: BaseNum + Copy + 'static> bytemuck::Pod for Rect<T> {}
