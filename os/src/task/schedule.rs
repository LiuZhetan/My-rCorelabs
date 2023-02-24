use core::cmp::Ordering;

const BIG_STRIDE: u8 = 255;
const MIN_DIFF:u8 = BIG_STRIDE / 2;

#[derive(Copy,Clone)]
pub struct Stride(u8);

impl Stride {
    pub fn new(s:u8) -> Stride {
        Stride {
            0:s
        }
    }
    
    pub fn update_stride(&mut self, p:u8) {
        let pass = BIG_STRIDE / p;
        self.0 += pass;
    }
}

impl PartialEq<Self> for Stride {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0 == other.0 {
            Some(Ordering::Equal)
        }
        else if self.0 > other.0 { 
            let diff = self.0 - other.0;
            if diff <= MIN_DIFF {
                Some(Ordering::Greater)
            }
            else { 
                Some(Ordering::Less)
            }
        }
        else {
            let diff = other.0 - self.0;
            if diff <= MIN_DIFF {
                Some(Ordering::Less)
            }
            else {
                Some(Ordering::Greater)
            }
        }
    }
}