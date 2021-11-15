use rand::{Rng, prelude::ThreadRng};

use crate::localization::{Localization, Localize};

#[derive(Clone, Copy, Debug)]
pub struct Health {
    current: isize,
    max: isize,
}

impl Health {
    fn new(current: isize, max: isize) -> Self {
        Health {
            current,
            max,
        }
    }

    fn reduce(&mut self, amount: isize) {
        self.current -= amount
    }

    pub fn alive(&self) -> bool {
        self.current > 0
    }

    pub fn current(&self) -> isize {
        self.current
    }

    pub fn max(&self) -> isize {
        self.max
    }
}

impl Localize for Health {
    fn localize(&self, localization: &Localization) -> String {
        format!("{}: {}/{}", localization.health, self.current, self.max)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vitality {
    health: Health,
    dodge: isize,
}

impl Vitality {
    fn new(health: Health, dodge: isize) -> Self {
        Vitality {
            health,
            dodge,
        }
    }

    pub fn health(&self) -> &Health {
        &self.health
    }

    pub fn dodge(&self) -> isize {
        self.dodge
    }

    pub fn modify_dodge(&mut self, modifier: isize) {
        self.dodge += modifier
    }

    fn take_attack(&mut self, damage: isize, accuracy: isize) {
        if accuracy > self.dodge {
            self.health.reduce(damage)
        }
    }
}

impl Localize for Vitality {
    fn localize(&self, localization: &Localization) -> String {
        format!("{}\t{} {}%", self.health.localize(localization), localization.dodge, self.dodge)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Attack {
    damage: isize,
    accuracy: isize,
}

impl Attack {
    fn new(damage: isize, accuracy: isize) -> Self {
        Attack {
            damage,
            accuracy,
        }
    }

    pub fn damage(&self) -> isize {
        self.damage
    }

    pub fn accuracy(&self) -> isize {
        self.accuracy
    }

    pub fn reduce_accuracy(&mut self, amount: isize) {
        self.accuracy -= amount
    }

    fn attack(&self, target: &mut Vitality) {
        target.take_attack(self.damage, self.accuracy)
    }
}

impl Localize for Attack {
    fn localize(&self, localization: &Localization) -> String {
        format!("{} {}\t{} {}%", localization.attack, self.damage, localization.accuracy, self.accuracy)
    }
}

#[derive(Clone, Debug)]
pub struct Random {
    rng: ThreadRng,
}

impl Random {
    pub fn new() -> Self {
        Random {
            rng: rand::thread_rng(),
        }
    }

    pub fn collide(&mut self, attack: &Attack, vitality: &mut Vitality) {
        let attack_roll = Attack::new(
            attack.damage,
            attack.accuracy - self.rng.gen_range(0..100),
        );
        attack_roll.attack(vitality)
    }

    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.rng.gen_range(0..items.len())]
    }

    pub fn choose_mut<'a, T>(&mut self, items: &'a mut [T]) -> &'a mut T {
        &mut items[self.rng.gen_range(0..items.len())]
    }
}

#[derive(Clone, Debug)]
pub struct PassiveEntity {
    vitality: Vitality,
}

impl PassiveEntity {
    pub fn vitality(&self) -> &Vitality {
        &self.vitality
    }

    pub fn vitality_mut(&mut self) -> &mut Vitality {
        &mut self.vitality
    }
}

#[derive(Clone, Debug)]
pub struct AggressiveEntity {
    vitality: Vitality,
    attack: Attack,
}

impl AggressiveEntity {
    pub fn vitality(&self) -> &Vitality {
        &self.vitality
    }

    pub fn vitality_mut(&mut self) -> &mut Vitality {
        &mut self.vitality
    }

    pub fn attack(&self) -> &Attack {
        &self.attack
    }

    pub fn attack_mut(&mut self) -> &mut Attack {
        &mut self.attack
    }
}

#[derive(Clone, Debug)]
pub enum Entity {
    Passive(PassiveEntity),
    Aggressive(AggressiveEntity),
}

impl Entity {
    pub fn passive(max_health: isize, dodge: isize) -> PassiveEntity {
        PassiveEntity {
            vitality: Entity::full_health(max_health, dodge),
        }
    }

    pub fn aggressive(max_health: isize, dodge: isize, attack_damage: isize, accuracy: isize) -> AggressiveEntity {
        AggressiveEntity {
            vitality: Entity::full_health(max_health, dodge),
            attack: Attack::new(attack_damage, accuracy),
        }
    }

    fn full_health(max_health: isize, dodge: isize) -> Vitality {
        Vitality::new(
            Health::new(max_health, max_health),
            dodge,
        )
    }
}