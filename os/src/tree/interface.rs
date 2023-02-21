use crate::tree::StrongRef;

#[derive(Copy, Clone)]
pub enum RBColor {
    Black,
    Red,
}

impl RBColor {
    pub(crate) fn is_black(&self) -> bool {
        match self {
            RBColor::Black => true,
            RBColor::Red => false
        }
    }

    pub(crate) fn is_red(&self) -> bool {
        match self {
            RBColor::Black => false,
            RBColor::Red => true
        }
    }
}

impl PartialEq for RBColor {
    fn eq(&self, other: &Self) -> bool {
        (self.is_black() && other.is_black()) ||
            (self.is_red() && other.is_red())
    }
}

/*pub trait BranchType {
    type Item;
    //fn strong_ref(self) -> StrongRef<Self::Item>;
}*/

pub trait NodeAttr {
    type Key: PartialOrd + Clone;
    type Value: Clone;
    fn new(key:Self::Key, value:Self::Value, color:RBColor) -> Self;
    fn key(&self) -> Self::Key;
    fn value(&self) -> Self::Value;
    fn set_key(&mut self, key: Self::Key);
    fn set_value(&mut self, value: Self::Value);
}

pub trait ModifyColor {
    fn color(&self) -> RBColor;
    fn set_color(&mut self, color:RBColor);
}

pub trait Branch<T> {
    fn parent(&self) -> StrongRef<T>;
    fn left(&self) -> StrongRef<T>;
    fn right(&self) -> StrongRef<T>;
    fn set_parent(&mut self, p:StrongRef<T>);
    fn set_left(&mut self, left:StrongRef<T>);
    fn set_right(&mut self, right:StrongRef<T>);
}

pub trait RBBranch<T> : Branch<T> + ModifyColor + NodeAttr {

}