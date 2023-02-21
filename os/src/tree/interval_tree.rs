use crate::tree::{GeneralRef, StrongRef};
use crate::tree::red_black_map::{BaseMap, BranchNode, RBNode};
use core::cmp::Ordering;
use crate::tree::red_black_map::RBNode::Nil;

pub trait Interval {
    type Item: PartialOrd + Clone;
    fn low(&self) -> Self::Item;
    fn high(&self) -> Self::Item;
}

#[derive(Copy,Clone,Debug)]
pub struct IntervalCell<T:Interval> (T);

impl<T> IntervalCell<T>
    where T: Interval {
    fn overlap(x:&IntervalCell<T>, y:&IntervalCell<T>) -> bool {
        x.0.low() <= y.0.high() && y.0.low() <= x.0.high()
    }

    pub fn new(x: T) -> Self {
        Self {
            0:x
        }
    }

    pub fn low(&self) -> <T as Interval>::Item {
        self.0.low()
    }

    pub fn high(&self) -> <T as Interval>::Item {
        self.0.high()
    }
}

impl<T> PartialEq<Self> for IntervalCell<T>
    where T: Interval {
    fn eq(&self, other: &Self) -> bool {
        self.0.low() == other.0.low()
    }
}

impl<T> PartialOrd for IntervalCell<T>
    where T: Interval {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return if self.0.low() > other.0.low() {
            Some(Ordering::Greater)
        } else if self.0.low() < other.0.low() {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

type IntervalNode<T> = BranchNode<IntervalCell<T>,<T as Interval>::Item>;
type IntRef<T> = StrongRef<RBNode<IntervalNode<T>>>;

pub struct IntervalMap<T: Interval + Clone>(BaseMap<IntervalNode<T>>);
unsafe impl<T: Interval + Clone> Sync for IntervalMap<T> {}

impl<T> IntervalMap<T>
    where T: Interval + Clone {
    fn update_func(x:<T as Interval>::Item,
                   y:<T as Interval>::Item,
                   z:<T as Interval>::Item)
                   -> <T as Interval>::Item {
        let mut max = if y > z { y } else { z };
        if x > max {
            max = x;
        }
        max
    }

    fn update_value(mut x:IntRef<T>) {
        let mut left_val;
        let mut right_val;
        let mut max_val;
        while x.is_coins() {
            let x_key = x.key().unwrap();
            let x_val = x_key.high();
            left_val = match x.left() {
                Some(inner) => {
                    if inner.is_coins() {
                        inner.value().unwrap()
                    }
                    else {
                        x_val.clone()
                    }
                },
                None => x_val.clone()
            };
            right_val = match x.right() {
                Some(inner) => {
                    if inner.is_coins() {
                        inner.value().unwrap()
                    }
                    else {
                        x_val.clone()
                    }
                },
                None => x_val.clone()
            };

            max_val = Self::update_func(x_val,left_val,right_val);
            x.try_set_value(max_val);
            x = x.p()
        }
    }
}

impl<T> IntervalMap<T>
    where T: Interval + Clone {
    fn interval_search_node(&self, key: IntervalCell<T>) -> IntRef<T> {
        let mut x = self.0.root.clone_strong();
        while x.is_coins() && !IntervalCell::overlap(&x.key().unwrap(), &key) {
            let left = x.left();
            if left.is_some() {
                let left_x = left.unwrap();
                if left_x.is_coins() && left_x.value().unwrap() >= key.low() {
                    x = x.left().unwrap();
                }
                else {
                    x = x.right().unwrap();
                }
            }
        }
        x
    }

    pub fn interval_search(&self, key: T) -> Option<T> {
        let key = IntervalCell::new(key);
        let res = self.interval_search_node(key);
        if res.is_coins() {
            //return res.key()
            match res.key() {
                Some(t) => {return Some(t.0);}
                None => {return None;}
            }
        }
        else {
            return None
        }
    }

    pub fn interval_insert(&mut self, key: T) {
        let key = IntervalCell::new(key);
        let h = key.high();
        let x = StrongRef::new(RBNode::new(key,h));
        if self.0.try_inseart_node(x.clone_strong()) {
            self.0.size +=1;
        }
    }

    /// 删除与key重复的节点
    pub fn interval_delete(&mut self, key: T) -> Option<<T as Interval>::Item> {
        let key = IntervalCell::new(key);
        let to_delete = self.0.search_node(key);
        if to_delete.is_nil() {
            panic!("can not found key ");
        }
        let res_node = self.0.delete_node(to_delete);
        self.0.size -= 1;
        if self.0.size == 0 {
            // 重置根节点
            self.0.root = StrongRef::new(Nil(None));
        }
        res_node.value()
    }
}

impl<T> IntervalMap<T>
    where T: Interval + Clone {
    pub fn new() -> Self {
        let mut res = Self { 0: BaseMap::new() };
        res.0.set_update_node_func(Self::update_value);
        res
    }
    pub fn clear(&mut self) {
        self.0.root = StrongRef::new(RBNode::nil_none());
        self.0.size = 0;
    }

    pub fn size(&self) -> usize {
        self.0.size
    }
}


