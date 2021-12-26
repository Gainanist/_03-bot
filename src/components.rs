use enum_map::Enum;

use crate::{dice::IDiceRoll, localization::{Localization, RenderText}};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Health {
    current: usize,
    max: usize,
}

impl Health {
    pub fn new(max: usize) -> Self {
        Health {
            current: max,
            max,
        }
    }

    pub fn reduce(&mut self, amount: usize) {
        if amount > self.current {
            self.current = 0
        } else {
            self.current -= amount
        }
    }

    pub fn alive(&self) -> bool {
        self.current > 0
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn max(&self) -> usize {
        self.max
    }
}

impl RenderText for Health {
    fn render_text(&self, _localization: &Localization) -> String {
        let current_health = String::from("▮")
            .repeat(self.current());
        let empty_health_char = match self.alive() {
            true => "▯",
            false => "X",
        };
        let empty_health = String::from(empty_health_char)
            .repeat(self.max() - self.current());

        format!("[{}{}]", current_health, empty_health)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Vitality {
    health: Health,
    dodge: isize,
}

impl Vitality {
    pub fn new(max_health: usize, dodge: isize) -> Self {
        Vitality {
            health: Health::new(max_health),
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

    pub fn take_attack(&mut self, damage: usize, accuracy: isize) {
        if accuracy > self.dodge {
            self.health.reduce(damage)
        }
    }
}

impl RenderText for Vitality {
    fn render_text(&self, localization: &Localization) -> String {
        if self.health.alive() {
            format!("{} - {}%", self.health.render_text(localization), 100_usize.saturating_sub(self.dodge.max(0) as usize))
        } else {
            self.health.render_text(localization)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Attack {
    damage: usize,
    accuracy: isize,
}

impl Attack {
    pub fn new(damage: usize, accuracy: isize) -> Self {
        Attack {
            damage,
            accuracy,
        }
    }

    pub fn modify_accuracy(&mut self, modifier: isize) {
        self.accuracy += modifier
    }

    pub fn attack(&self, target: &mut Vitality, dice_roll: IDiceRoll) {
        target.take_attack(self.damage, self.accuracy + dice_roll.0)
    }
}

impl RenderText for Attack {
    fn render_text(&self, localization: &Localization) -> String {
        format!("{} {}, {}%", localization.attack, self.damage, self.accuracy)
    }
}

#[derive(Clone, Copy, Debug, Enum, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BygonePart {
    Core,
    Sensor,
    Gun,
    LeftWing,
    RightWing,
}

#[derive(Clone, Copy, Debug, Enum, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Bygone03Stage {
    Armored,
    Exposed,
    Burning,
    Defeated,
}

impl Bygone03Stage {
    pub fn next(self) -> Self {
        match self {
            Self::Armored => Self::Exposed,
            Self::Exposed => Self::Burning,
            Self::Burning => Self::Defeated,
            Self::Defeated => Self::Defeated,
        }
    }

    pub fn terminal(&self) -> bool {
        *self == Self::Defeated
    }
}

impl RenderText for Bygone03Stage {
    fn render_text(&self, localization: &Localization) -> String {
        match *self {
            Self::Armored => &localization.core_armored,
            Self::Exposed => &localization.core_exposed,
            Self::Burning => &localization.core_burning,
            Self::Defeated => &localization.core_destroyed,
        }.clone()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Enemy;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Player;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Active;

