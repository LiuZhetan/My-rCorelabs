use core::cmp::Ordering;

const BIG_STRIDE: u8 = 255;

struct Stride(u8);

impl PartialEq<Self> for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let res = self.0 - other.0;
        let min_diff = BIG_STRIDE / 2;
        if res >= 0 {

        }
        else {

        }
    }
}