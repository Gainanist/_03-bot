use bevy_rng::Rng as BevyRng;
use rand::Rng;

#[derive(Clone, Copy, Debug, Default)]
pub struct IDiceRoll(pub isize);

#[derive(Clone, Copy, Debug, Default)]
pub struct UDiceRoll(pub usize);

pub trait Dice {
    fn iroll(&mut self, min: isize, max: isize) -> IDiceRoll;
    fn uroll(&mut self, max: usize) -> UDiceRoll {
        UDiceRoll(self.iroll(0, max as isize).0 as usize)
    }

    fn choose<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.len() == 0 {
            None
        } else {
            Some(&items[self.uroll(items.len()).0])
        }
    }

    fn choose_mut<'a, T>(&mut self, items: &'a mut [T]) -> Option<&'a mut T> {
        if items.len() == 0 {
            None
        } else {
            Some(&mut items[self.uroll(items.len()).0])
        }
    }
}

impl Dice for BevyRng {
    fn iroll(&mut self, min: isize, max: isize) -> IDiceRoll {
        IDiceRoll(self.gen_range(min..max))
    }
}
