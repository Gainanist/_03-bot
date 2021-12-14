use bevy_rng::Rng as BevyRng;
use rand::Rng;

#[derive(Clone, Copy, Debug, Default)]
pub struct DiceRoll(pub usize);

pub trait Dice {
    fn roll(&mut self, max: usize) -> DiceRoll;

    fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.roll(items.len()).0]
    }

    fn choose_mut<'a, T>(&mut self, items: &'a mut [T]) -> &'a mut T {
        &mut items[self.roll(items.len()).0]
    }
}

impl Dice for BevyRng {
    fn roll(&mut self, max: usize) -> DiceRoll {
        DiceRoll(self.gen_range(0..max))
    }
}
