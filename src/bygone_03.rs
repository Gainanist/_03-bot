use crate::{entities::{AggressiveEntity, Attack, Entity, PassiveEntity, Random, Vitality}, localization::{Localization, Localize}};

#[derive(Copy, Clone, Debug, PartialEq)]
enum Stage {
    Armored,
    Exposed,
    Burning,
    Defeated,
}

impl Stage {
    fn next(self) -> Self {
        match self {
            Self::Armored => Self::Exposed,
            Self::Exposed => Self::Burning,
            Self::Burning => Self::Defeated,
            Self::Defeated => Self::Defeated,
        }
    }

    fn terminal(&self) -> bool {
        *self == Self::Defeated
    }
}

impl Localize for Stage {
    fn localize(&self, localization: &Localization) -> String {
        match *self {
            Self::Armored => &localization.core_armored,
            Self::Exposed => &localization.core_exposed,
            Self::Burning => &localization.core_burning,
            Self::Defeated => &localization.core_destroyed,
        }.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Bygone03 {
    core: AggressiveEntity,
    sensor: PassiveEntity,
    gun: PassiveEntity,
    left_wing: PassiveEntity,
    right_wing: PassiveEntity,
    stage: Stage,
}

impl Bygone03 {
    pub fn normal() -> Self {
        Bygone03 {
            core: Entity::aggressive(1, 80, 1, 100),
            sensor: Entity::passive(1, 70),
            gun: Entity::passive(1, 50),
            left_wing: Entity::passive(1, 30),
            right_wing: Entity::passive(1, 30),
            stage: Stage::Armored,
        }
    }

    pub fn alive(&self) -> bool {
        !self.stage.terminal()
    }

    pub fn attack(&self) -> &Attack {
        &self.core.attack()
    }

    pub fn damage_core(&mut self, attack: &Attack, rng: &mut Random) {
        if self.try_destroy_part(&mut |bygone: &mut Bygone03| bygone.core.vitality_mut(), attack, rng) {
            self.advance_stage();
        }
    }

    fn advance_stage(&mut self) {
        self.core = Entity::aggressive(
            self.core.vitality().health().max(),
            self.core.vitality().dodge(),
            self.core.attack().damage(),
            self.core.attack().accuracy(),
        );
        self.stage = self.stage.next();
    }

    pub fn damage_sensor(&mut self, attack: &Attack, rng: &mut Random) {
        if self.try_destroy_part(&mut |bygone: &mut Bygone03| bygone.sensor.vitality_mut(), attack, rng) {
            self.core.attack_mut().reduce_accuracy(40);
        }
    }

    pub fn damage_gun(&mut self, attack: &Attack, rng: &mut Random) {
        if self.try_destroy_part(&mut |bygone: &mut Bygone03| bygone.gun.vitality_mut(), attack, rng) {
            self.core.attack_mut().reduce_accuracy(30);
        }
    }

    pub fn damage_left_wing(&mut self, attack: &Attack, rng: &mut Random) {
        if self.try_destroy_part(&mut |bygone: &mut Bygone03| bygone.left_wing.vitality_mut(), attack, rng) {
            self.modify_dodge(-10)
        }
    }

    pub fn damage_right_wing(&mut self, attack: &Attack, rng: &mut Random) {
        if self.try_destroy_part(&mut |bygone: &mut Bygone03| bygone.right_wing.vitality_mut(), attack, rng) {
            self.modify_dodge(-10)
        }
    }

    fn modify_dodge(&mut self, modifier: isize) {
        self.core.vitality_mut().modify_dodge(modifier);
        self.sensor.vitality_mut().modify_dodge(modifier);
        self.gun.vitality_mut().modify_dodge(modifier);
        self.left_wing.vitality_mut().modify_dodge(modifier);
        self.right_wing.vitality_mut().modify_dodge(modifier);
    }

    fn try_destroy_part<F>(&mut self, choose_part: &mut F, attack: &Attack, rng: &mut Random) -> bool
        where F : Fn(&mut Bygone03)->&mut Vitality
    {
        if !self.alive() {
            return false;
        }

        let part = choose_part(self);
        if !part.health().alive() {
            return false;
        }

        rng.collide(attack, part);

        !part.health().alive()
    }

    // fn try_destroy_part1(&self, vitality: &mut Vitality, attack: &Attack, rng: &mut Random) -> bool {
    //     if self.stage.terminal() {
    //         return false;
    //     }

    //     if !vitality.health().alive() {
    //         return false;
    //     }

    //     rng.collide(attack, vitality);

    //     !vitality.health().alive()
    // }
}

impl Localize for Bygone03 {
    fn localize(&self, localization: &Localization) -> String {
        format!(
"
{}

{}: {} - {}
{}: {}
{}: {}
{}: {}
{}: {}
",
            self.attack().localize(localization),
            localization.core, self.core.vitality().localize(localization), self.stage.localize(localization),
            localization.sensor, self.sensor.vitality().localize(localization),
            localization.left_wing, self.left_wing.vitality().localize(localization),
            localization.right_wing, self.right_wing.vitality().localize(localization),
            localization.gun, self.gun.vitality().localize(localization),
        )
    }
}
