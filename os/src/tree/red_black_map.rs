use crate::tree::interface::{Branch, NodeAttr, ModifyColor, RBColor, RBBranch};
use crate::tree::{GeneralRef, StrongRef, WeakRef};
use crate::tree::red_black_map::RBNode::{Coins, Nil};

// 分支节点
pub struct BranchNode<K:PartialOrd + Clone,V: Clone> {
    color:RBColor,
    key:K,
    value:V,
    parent:WeakRef<RBNode<Self>>,
    left:StrongRef<RBNode<Self>>,
    right:StrongRef<RBNode<Self>>,
}

impl<K,V> PartialEq for BranchNode<K,V>
    where K:PartialOrd + Clone, V:Clone{
    fn eq(&self, other: &Self) -> bool {
        let res = self as *const BranchNode<K,V> as usize == other as *const BranchNode<K,V> as usize;
        // println!("{}",res);
        res
    }
}

impl<K, V> RBBranch<RBNode<Self>> for BranchNode<K, V> where K: Clone + PartialOrd, V: Clone {

}

impl<K,V> Branch<RBNode<Self>> for BranchNode<K, V>
    where K:PartialOrd + Clone, V:Clone {

    fn parent(&self) -> StrongRef<RBNode<Self>>{
        match self.parent.to_strong() {
            Some(ptr) => ptr,
            None => StrongRef::new(Nil(None))
        }
    }

    fn left(&self) -> StrongRef<RBNode<Self>> {
        StrongRef::clone(&self.left)
    }

    fn right(&self) -> StrongRef<RBNode<Self>> {
        StrongRef::clone(&self.right)
    }

    fn set_parent(&mut self, p:StrongRef<RBNode<Self>>) {
        self.parent = WeakRef::new(p);
    }

    fn set_left(&mut self, left:StrongRef<RBNode<Self>>) {
        self.left = left;
    }

    fn set_right(&mut self, right:StrongRef<RBNode<Self>>) {
        self.right = right;
    }
}

impl<K,V> NodeAttr for BranchNode<K, V>
    where K:PartialOrd + Clone, V:Clone{
    type Key = K;
    type Value = V;

    // 新建一个独立的节点,parent,left,right都是nil(None)
    fn new(key:K, value: V, color:RBColor) -> Self{
        Self {
            color,
            key,
            value,
            parent:WeakRef::new(StrongRef::new(RBNode::nil_none())),
            left:StrongRef::new(RBNode::nil_none()),
            right:StrongRef::new(RBNode::nil_none()),
        }
    }

    fn key(&self) -> K {
        self.key.clone()
    }

    fn value(&self) -> V {
        self.value.clone()
    }

    fn set_key(&mut self, key:K) {
        self.key = key
    }

    fn set_value(&mut self, value:V) {
        self.value = value
    }
}

impl<K,V> ModifyColor for BranchNode<K, V>
where K: PartialOrd + Clone, V: Clone{
    fn color(&self) -> RBColor{
        self.color
    }
    fn set_color(&mut self, color:RBColor) {
        self.color = color;
    }
}



//叶子节点
pub struct LeafNode<T: RBBranch<RBNode<T>>> {
    parent:WeakRef<RBNode<T>>,
    color:RBColor,
}

impl<T> LeafNode<T>
where T: RBBranch<RBNode<T>> {
    pub fn new(p:StrongRef<RBNode<T>>) -> Self{
        LeafNode {
            parent:WeakRef::new(p),
            color:RBColor::Black,
        }
    }

    fn parent(&self) -> StrongRef<RBNode<T>> {
        match self.parent.to_strong() {
            Some(ptr) => ptr,
            None => StrongRef::new(Nil(None))
        }
    }

    fn set_parent(&mut self, p:StrongRef<RBNode<T>>) {
        self.parent = WeakRef::new(p);
    }
}

impl<T> ModifyColor for LeafNode<T>
where T: RBBranch<RBNode<T>> {
    fn color(&self) -> RBColor {
        self.color
    }

    fn set_color(&mut self, color: RBColor) {
        self.color = color;
    }
}

// 红黑树节点
pub enum RBNode<T: RBBranch<Self>> {
    Coins(T),
    Nil(Option<LeafNode<T>>),
}

impl<T> PartialEq for RBNode<T>
where T: RBBranch<Self>{
    fn eq(&self, other: &Self) -> bool {
        let res = self as *const RBNode<T> as usize == other as *const RBNode<T> as usize;
        // println!("{}",res);
        res
    }
}

impl<T> RBNode<T>
where T: RBBranch<Self>{
    pub fn new(key:<T as NodeAttr>::Key,value:<T as NodeAttr>::Value) -> Self{
        Coins(T::new(key,value,RBColor::Red))
    }

    pub fn nil_none() -> RBNode<T> {
        Nil(None)
    }

    pub fn nil_leaf(p:StrongRef<RBNode<T>>) -> RBNode<T> {
        Nil(Some(LeafNode::new(p)))
    }

    pub fn is_coins(&self) -> bool{
        match self {
            Coins(..) => true,
            _ => false,
        }
    }

    pub fn is_nil(&self) -> bool{
        match self {
            Coins(..) => false,
            _ => true,
        }
    }

    pub fn key(&self) -> Option<<T as NodeAttr>::Key>{
        match self {
            Coins(branch) => {
                Some(branch.key())
            },
            Nil(_) => None,
        }
    }

    pub fn value(&self) -> Option<<T as NodeAttr>::Value>{
        match self {
            Coins(branch) => {
                Some(branch.value())
            },
            Nil(_) => None,
        }
    }

    pub fn try_set_key(&mut self, key:<T as NodeAttr>::Key) {
        match self {
            Coins(inner) => {
                inner.set_key(key);
            },
            Nil(_) => panic!("can not set key for Nil")
        }
    }

    pub fn try_set_value(&mut self, value:<T as NodeAttr>::Value) {
        match self {
            Coins(inner) => {
                inner.set_value(value);
            },
            Nil(_) => panic!("can not set value for Nil")
        }
    }

    pub fn p(&self) -> StrongRef<Self> {
        match self {
            Coins(branch) => branch.parent(),
            Nil(leaf) => {
                match leaf {
                    Some(inner) => inner.parent(),
                    None => StrongRef::new(Nil(None))
                }
            },
        }
    }

    pub fn color(&self) -> RBColor {
        match self {
            Coins(branch) => branch.color(),
            Nil(_) => RBColor::Black,
        }
    }

    pub fn left(&self) -> Option<StrongRef<Self>> {
        match self {
            Coins(branch) => Some(branch.left()),
            Nil(_) => None,
        }
    }

    pub fn right(&self) -> Option<StrongRef<Self>> {
        match self {
            Coins(branch) => Some(branch.right()),
            Nil(_) => None,
        }
    }

    fn try_set_color(&mut self, color: RBColor) {
        match self {
            Coins(branch) => branch.set_color(color),
            Nil(_) => panic!("cant not set color for Nil"),
        }
    }

    fn try_set_p(&mut self, p:StrongRef<Self>) {
        match self {
            Coins(branch) => branch.set_parent(p),
            Nil(nil) => match nil {
                Some(leaf) => leaf.set_parent(p),
                None => panic!("cant not set parent for None")
            },
        }
    }

    fn try_set_left(&mut self, left:StrongRef<Self>) {
        match self {
            Coins(branch) => branch.set_left(left),
            Nil(_) => panic!("cant not set right for Nil"),
        }
    }

    fn try_set_right(&mut self, right:StrongRef<Self>) {
        match self {
            Coins(branch) => branch.set_right(right),
            Nil(_) => panic!("cant not set right for Nil"),
        }
    }
}

pub struct BaseMap<T>
where T:RBBranch<RBNode<T>> {
    pub root:StrongRef<RBNode<T>>,
    pub size:usize,
    /*update_func:Option<fn(x:<T as NodeAttr>::Value,
                          left_x:<T as NodeAttr>::Value,
                          right_x:<T as NodeAttr>::Value)
                          -> <T as NodeAttr>::Value>*/
    update_node_func: Option<fn(x: StrongRef<RBNode<T>>)>
}

impl<T> BaseMap<T>
    where T:RBBranch<RBNode<T>> {
    pub fn new() -> Self {
        Self {
            root: StrongRef::new(RBNode::nil_none()),
            size:0,
            update_node_func:None
        }
    }

    fn exchange_order(&mut self,
                      x:StrongRef<RBNode<T>>,
                      y:StrongRef<RBNode<T>>) {
        let mut p_x = x.p();
        let mut y = y.clone_strong();
        y.try_set_p(p_x.clone_strong());
        if p_x.is_nil() {
            self.root = y.clone_strong();
        }
        else if x == p_x.left().unwrap() {
            p_x.try_set_left(y);
        }
        else {
            p_x.try_set_right(y);
        }
    }

    /*pub fn set_update_func(&mut self, func: fn(<T as NodeAttr>::Value,
                                               <T as NodeAttr>::Value,
                                               <T as NodeAttr>::Value) -> <T as NodeAttr>::Value) {
        self.update_func = Some(func);
    }*/

    pub fn set_update_node_func(&mut self, func: fn(x: StrongRef<RBNode<T>>)) {
        self.update_node_func = Some(func);
    }

    fn update_path(&self, mut x:StrongRef<RBNode<T>>) {
        let f = self.update_node_func.unwrap();
        while x.is_coins() {
            //self.update_node_value(x.clone_strong());
            f(x.clone_strong());
            x = x.p()
        }
    }

    fn left_rotate(&mut self, x: StrongRef<RBNode<T>>) {
        let mut y= x.right().unwrap();
        let mut x = x.clone_strong();
        x.try_set_right(y.left().unwrap());
        y.left().unwrap().try_set_p(x.clone_strong());
        y.try_set_p(x.p());
        self.exchange_order(x.clone_strong(),
                            y.clone_strong());
        y.try_set_left(x.clone_strong());
        x.try_set_p(y.clone_strong());

        if let Some(f) = self.update_node_func {
            y.try_set_value(x.value().unwrap());
            f(x);
        }
    }

    fn right_rotate(&mut self, x: StrongRef<RBNode<T>>) {
        let mut y= x.left().unwrap();
        let mut x = x.clone_strong();
        x.try_set_left(y.right().unwrap());
        y.right().unwrap().try_set_p(x.clone_strong());
        y.try_set_p(x.p());
        self.exchange_order(x.clone_strong(),
                            y.clone_strong());
        y.try_set_right(x.clone_strong());
        x.try_set_p(y.clone_strong());

        if let Some(f) = self.update_node_func {
            y.try_set_value(x.value().unwrap());
            f(x);
        }
    }
}

impl<T> BaseMap<T>
    where T:RBBranch<RBNode<T>> {
    fn tree_minimum(x:&StrongRef<RBNode<T>>) -> StrongRef<RBNode<T>>{
        let mut x = x.clone_strong();
        loop {
            let left = x.left().unwrap();
            if left.is_coins() {
                x = left;
            }
            else {
                break
            }
        }
        x
    }

    fn tree_maximum(x:&StrongRef<RBNode<T>>) -> StrongRef<RBNode<T>>{
        let mut x = x.clone_strong();
        loop {
            let right = x.right().unwrap();
            if right.is_coins() {
                x = right;
            }
            else {
                break
            }
        }
        x
    }

    fn tree_predecessor(x:&StrongRef<RBNode<T>>) -> StrongRef<RBNode<T>> {
        let mut x = x.clone_strong();
        let left_x = x.left().unwrap();
        if left_x.is_coins() {
            return Self::tree_maximum(&left_x);
        }
        let mut y = x.p();
        while y.is_coins() && x == y.left().unwrap() {
            x = y.clone_strong();
            y = y.p();
        }
        y
    }

    fn tree_successor(x:&StrongRef<RBNode<T>>) -> StrongRef<RBNode<T>> {
        let mut x = x.clone_strong();
        let right_x = x.right().unwrap();
        if right_x.is_coins() {
            return Self::tree_minimum(&right_x);
        }
        let mut y = x.p();
        while y.is_coins() && x == y.right().unwrap() {
            x = y.clone_strong();
            y = y.p();
        }
        y
    }

    pub(crate) fn search_node(&self, key: <T as NodeAttr>::Key) -> StrongRef<RBNode<T>> {
        let mut x = self.root.clone_strong();
        while x.is_coins() {
            if key == x.key().unwrap() {
                break
            }
            else if key < x.key().unwrap() {
                x = x.left().unwrap();
            }
            else {
                x = x.right().unwrap();
            }
        }
        x
    }

    fn rb_insert_fixup(&mut self, z: StrongRef<RBNode<T>>) {
        let mut z = z.clone_strong();
        loop {
            if z.p().color().is_black() {
                break
            }
            let mut parent_z = z.p();
            let mut grad_z = parent_z.p();
            if parent_z == grad_z.left().unwrap() {
                //左三种情况
                // y为叔叔
                let mut y = grad_z.right().unwrap();
                if y.color().is_red() {
                    //case 1：z的父亲和叔叔都是红色
                    parent_z.try_set_color(RBColor::Black);
                    y.try_set_color(RBColor::Black);
                    grad_z.try_set_color(RBColor::Red);
                    z = grad_z.clone_strong();
                }
                else {
                    if z == parent_z.right().unwrap() {
                        //case 2：叔叔黑色,z为父亲的右孩子,左旋
                        z = parent_z.clone_strong();
                        self.left_rotate(z.clone_strong());
                    }
                    //case 3：叔叔黑色,z为父亲的左孩子,染色，右旋
                    let mut p_z = z.p();
                    let mut grad_z = p_z.p();
                    p_z.try_set_color(RBColor::Black);
                    grad_z.try_set_color(RBColor::Red);
                    self.right_rotate(grad_z);
                }
            }
            else {
                //右三种情况
                // y为叔叔
                let mut y = grad_z.left().unwrap();
                if y.color().is_red() {
                    //case 1：z的父亲和叔叔都是红色
                    parent_z.try_set_color(RBColor::Black);
                    y.try_set_color(RBColor::Black);
                    grad_z.try_set_color(RBColor::Red);
                    z = grad_z.clone_strong();
                }
                else {
                    if z == parent_z.left().unwrap() {
                        //case 2：叔叔黑色,z为父亲的左孩子,右旋
                        z = parent_z.clone_strong();
                        self.right_rotate(z.clone_strong());
                    }
                    //case 3：叔叔黑色,z为父亲的右孩子,染色,左旋
                    let mut p_z = z.p();
                    let mut grad_z = p_z.p();
                    p_z.try_set_color(RBColor::Black);
                    grad_z.try_set_color(RBColor::Red);
                    self.left_rotate(grad_z);
                }
            }
        }
        self.root.try_set_color(RBColor::Black);
    }

    pub(crate) fn try_inseart_node(&mut self, z: StrongRef<RBNode<T>>) -> bool{
        let mut z = z.clone_strong();
        //let z_node = Self::try_mut(&mut z);

        let mut y = StrongRef::new(RBNode::nil_none());
        let mut x = self.root.clone_strong();
        while x.is_coins() {
            y = x.clone_strong();
            if z.key() == x.key() {
                x.try_set_value(z.value().unwrap());
                return false;
            }
            else if z.key() < x.key() {
                x = x.left().unwrap();
            }
            else {
                x = x.right().unwrap();
            }
        }

        z.try_set_p(y.clone_strong());

        // 此时y为x的parent，x为需要替换的叶子节点
        if y.is_coins() {
            if z.key() < y.key() {
                y.try_set_left(z.clone_strong());
            }
            else {
                y.try_set_right(z.clone_strong());
            }
            // 如果有必要，则更新路径上的所有节点的value
            if self.update_node_func.is_some() {
                self.update_path(y.clone_strong());
            }
        }
        else {
            self.root = z.clone_strong();
        }
        // 增加两个叶子节点
        let mut z_2 = z.clone_strong();
        z_2.try_set_left(StrongRef::new(RBNode::nil_leaf(z.clone_strong())));
        z_2.try_set_right(StrongRef::new(RBNode::nil_leaf(z.clone_strong())));
        z_2.try_set_color(RBColor::Red);
        self.rb_insert_fixup(z_2);
        true
    }

    fn rb_delete_fixup(&mut self, x:StrongRef<RBNode<T>>) {
        let mut x = x.clone_strong();
        while x != self.root && x.color().is_black() {
            let mut p_x = x.p();
            if x == p_x.left().unwrap() {
                // x是左孩子
                let mut w = p_x.right().unwrap();
                if w.color().is_red() {
                    // case 1: w是红色的，左旋
                    println!("left case 1");
                    w.try_set_color(RBColor::Black);
                    p_x.try_set_color(RBColor::Red);
                    self.left_rotate(p_x.clone_strong());
                    w = p_x.right().unwrap();
                }

                let mut left_w = w.left().unwrap();
                let mut right_w = w.right().unwrap();
                if left_w.color().is_black() &&
                    right_w.color().is_black() {
                    // case 2: w的孩子都是黑色,w转红,x = p_x
                    println!("left case 2");
                    w.try_set_color(RBColor::Red);
                    x = p_x.clone_strong();
                }
                else {
                    if right_w.color().is_black() {
                        // case 3: 右孩子黑色,右旋w
                        println!("left case 3");
                        left_w.try_set_color(RBColor::Black);
                        w.try_set_color(RBColor::Red);
                        self.right_rotate(w.clone_strong());
                        w = p_x.right().unwrap();
                    }
                    // case 4:右孩子红，左旋p_x
                    println!("left case 4");
                    w.try_set_color(p_x.color());
                    p_x.try_set_color(RBColor::Black);
                    right_w.try_set_color(RBColor::Black);
                    self.left_rotate(p_x.clone_strong());
                    x = self.root.clone_strong();
                }
            }
            else {
                // x是右孩子
                let mut w = p_x.left().unwrap();
                if w.color().is_red() {
                    // case 1: w是红色的，右旋
                    println!("right case 1");
                    w.try_set_color(RBColor::Black);
                    p_x.try_set_color(RBColor::Red);
                    self.right_rotate(p_x.clone_strong());
                    w = p_x.left().unwrap();
                }

                let mut left_w = w.left().unwrap();
                let mut right_w = w.right().unwrap();
                if left_w.color().is_black() &&
                    right_w.color().is_black() {
                    // case 2: w的孩子都是黑色,w转红,x = p_x
                    println!("right case 2");
                    w.try_set_color(RBColor::Red);
                    x = p_x.clone_strong();
                }
                else {
                    if left_w.color().is_black() {
                        // case 3: 左孩子黑色,左旋w
                        println!("right case 3");
                        right_w.try_set_color(RBColor::Black);
                        w.try_set_color(RBColor::Red);
                        self.left_rotate(w.clone_strong());
                        w = p_x.left().unwrap();
                    }
                    // case 4:左孩子红，右旋p_x
                    println!("right case 4");
                    w.try_set_color(p_x.color());
                    p_x.try_set_color(RBColor::Black);
                    left_w.try_set_color(RBColor::Black);
                    self.right_rotate(p_x.clone_strong());
                    x = self.root.clone_strong();
                }
            }
        }
        if x.is_coins() {
            x.try_set_color(RBColor::Black);
        }
    }

    pub(crate) fn delete_node(&mut self, z: StrongRef<RBNode<T>>) -> StrongRef<RBNode<T>>{
        let mut z = z.clone_strong();
        // y或将取代z的位置
        let y;
        if z.left().unwrap().is_nil() ||
            z.right().unwrap().is_nil() {
            y = z.clone_strong();
        }
        else {
            y = Self::tree_successor(&z);
        }

        let left_y = y.left().unwrap();
        // x为y的后一个节点
        let mut x;
        if left_y.is_coins() {
            x = left_y;
        }
        else {
            x = y.right().unwrap();
        }

        let mut p_y = y.p();
        x.try_set_p(p_y.clone_strong());
        if p_y.is_nil() {
            self.root = x.clone_strong();
        }
        else if y == p_y.left().unwrap() {
            p_y.try_set_left(x.clone_strong());
        }
        else {
            p_y.try_set_right(x.clone_strong());
        }

        if y!=z {
            z.try_set_key(y.key().unwrap());
            z.try_set_value(y.value().unwrap());
        }

        if p_y.is_coins() {
            self.update_path(p_y);
        }

        if y.color().is_black() {
            self.rb_delete_fixup(x);
        }
        y
    }
}

pub struct RBTreeMap<K: PartialOrd + Clone, V: Clone>(BaseMap<BranchNode<K,V>>);

impl<K,V> RBTreeMap<K,V>
    where K: PartialOrd + Clone, V: Clone {
    fn search(&self, key: K) -> Option<V> {
        let res = self.0.search_node(key);
        if res.is_coins() {
            Some(res.value().unwrap())
        }
        else {
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        let x = StrongRef::new(RBNode::new(key,value));
        if self.0.try_inseart_node(x) {
            self.0.size +=1;
        }
    }

    pub fn delete(&mut self, key: K) -> Option<V> {
        let delete = self.0.search_node(key);
        if delete.is_nil() {
            panic!("can not found key ");
        }
        let res_node = self.0.delete_node(delete);
        self.0.size -= 1;
        if self.0.size == 0 {
            // 重置根节点
            self.0.root = StrongRef::new(Nil(None));
        }
        res_node.value()
    }
}

impl<K,V> RBTreeMap<K,V>
    where K: PartialOrd + Clone, V: Clone {
    pub fn new() -> Self {
        Self {
            0: BaseMap::new()
        }
    }

    pub fn clear(&mut self) {
        self.0.root = StrongRef::new(RBNode::nil_none());
        self.0.size = 0;
    }

    pub fn size(&self) -> usize {
        self.0.size
    }
}
