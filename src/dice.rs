

pub struct DiceRoll(pub usize);

pub trait Dice {
    pub fn roll(&mut self, max: usize) -> DiceRoll;

    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.rng.gen_range(0..items.len())]
    }

    pub fn choose_mut<'a, T>(&mut self, items: &'a mut [T]) -> &'a mut T {
        &mut items[self.rng.gen_range(0..items.len())]
    }
}

#[derive(Clone, Debug, Default)]
pub struct Dice {
    rng: ThreadRng,
}

impl Dice {
    pub fn new() -> Self {
        Dice {
            rng: rand::thread_rng(),
        }
    }

    pub fn roll(&mut self, max: usize) -> DiceRoll {
        DiceRoll(self.rng.gen_range(0..max))
    }

    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.rng.gen_range(0..items.len())]
    }

    pub fn choose_mut<'a, T>(&mut self, items: &'a mut [T]) -> &'a mut T {
        &mut items[self.rng.gen_range(0..items.len())]
    }
}
