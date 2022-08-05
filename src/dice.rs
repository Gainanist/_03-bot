use bevy_turborand::{GlobalRng};


pub trait Dice {
    fn d100(&mut self) -> isize;
}

impl Dice for GlobalRng {
    fn d100(&mut self) -> isize {
        self.isize(0..100)
    }
}

pub fn choose_mut<'a, T>(rng: &mut GlobalRng, items: &'a mut [T]) -> Option<&'a mut T> {
    if items.len() == 0 {
        None
    } else {
        Some(&mut items[rng.usize(0..items.len())])
    }
}
