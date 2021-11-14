use std::fmt::Display;

// pub trait Combatant {
//     fn take_damage(&mut self, damage: isize);
//     fn damage(&self) -> isize;
//     fn alive(&self) -> bool;

//     fn attack<R: Rng + ?Sized>(&self, rng: &mut R) -> (isize, isize) {
//         (self.damage(), rng.gen_range(0..100))
//     }
// }

pub struct HealthPool {
    max_health: isize,
    current_health: isize, 
}

impl HealthPool {
    pub fn full(max_health: isize) -> Self {
        HealthPool {
            max_health,
            current_health: max_health,
        }
    }

    pub fn damage(&mut self, damage: isize) {
        if damage > self.current_health {
            self.current_health = 0
        } else {
            self.current_health -= damage
        }
    }

    pub fn alive(&self) -> bool {
        self.current_health > 0
    }
}

pub struct Hero {
    health: HealthPool,
    hit_chance: isize,
    attack_damage: isize,
}

impl Hero {
    fn new() -> Self {
        Hero {
            health: HealthPool::full(6),
            hit_chance: 100,
            attack_damage: 1,
        }
    }

    pub fn damage(&mut self, damage: isize, hit_chance_roll: isize) {
        if hit_chance_roll < self.hit_chance as isize {
            self.health.damage(damage)
        }
    }

    pub fn alive(&self) -> bool {
        self.health.alive()
    }

    pub fn shoot(&self) -> isize {
        self.attack_damage
    }
}

pub struct BygoneBodyPart {
    hit_chance: isize,
    health: HealthPool,
}

impl BygoneBodyPart {
    fn new(hit_chance: isize, max_health: isize) -> Self {
        BygoneBodyPart {
            hit_chance,
            health: HealthPool::full(max_health),
        }
    }

    pub fn damage(&mut self, damage: isize, hit_chance_roll: isize) {
        if hit_chance_roll < self.hit_chance {
            self.health.damage(damage)
        }
    }

    pub fn alive(&self) -> bool {
        self.health.alive()
    }

    pub fn render(&self, dodge_penalty: isize) -> String {
        if self.alive() {
            (self.hit_chance + dodge_penalty).to_string() + "%"
        } else {
            "X".to_owned()
        }
    }
}

pub struct Bygone {
    sensor: BygoneBodyPart,
    core: BygoneBodyPart,
    left_wing: BygoneBodyPart,
    right_wing: BygoneBodyPart,
    gun: BygoneBodyPart,
    attack_damage: isize,
}

impl Bygone {
    fn new() -> Self {
        Bygone {
            sensor: BygoneBodyPart::new(30, 1),
            core: BygoneBodyPart::new(20, 3),
            left_wing: BygoneBodyPart::new(70, 1),
            right_wing: BygoneBodyPart::new(70, 1), 
            gun: BygoneBodyPart::new(50, 1),
            attack_damage: 1,
        }
    }

    pub fn alive(&self) -> bool {
        self.core.alive()
    }

    pub fn shoot(&self) -> isize {
        self.attack_damage
    }

    pub fn damage_sensor(&mut self, damage: isize, hit_chance_roll: isize) {
        self.sensor.damage(damage, hit_chance_roll - self.dodge_penalty());
    }

    pub fn damage_core(&mut self, damage: isize, hit_chance_roll: isize) {
        self.core.damage(damage, hit_chance_roll - self.dodge_penalty())
    }

    pub fn damage_left_wing(&mut self, damage: isize, hit_chance_roll: isize) {
        self.left_wing.damage(damage, hit_chance_roll - self.dodge_penalty())
    }

    pub fn damage_right_wing(&mut self, damage: isize, hit_chance_roll: isize) {
        self.right_wing.damage(damage, hit_chance_roll - self.dodge_penalty())
    }

    pub fn damage_gun(&mut self, damage: isize, hit_chance_roll: isize) {
        self.gun.damage(damage, hit_chance_roll - self.dodge_penalty())
    }

    fn dodge_penalty(&self) -> isize {
        (!self.left_wing.alive() as isize) * 10 + (!self.right_wing.alive() as isize) * 10
    }

    pub fn accuracy_penalty(&self) -> isize {
        (!self.sensor.alive() as isize) * 40 + (!self.gun.alive() as isize) * 30
    }
}

pub struct Battle {
    pub hero: Hero,
    pub bygone: Bygone,
}

impl Battle {
    pub fn new() -> Self {
        let mut battle = Battle {
            hero: Hero::new(),
            bygone: Bygone::new(),
        };
        battle.hero.health.current_health = 0;

        battle
    }

    pub fn start() -> Self {
        Battle {
            hero: Hero::new(),
            bygone: Bygone::new(),
        }
    }

    pub fn finished(&self) -> bool {
        !self.bygone.alive() || !self.hero.alive()
    }
}

const CORE_STATUS: [&str; 4] = [
    "УНИЧТОЖЕНО!",
    "ЯДРО ГОРИТ!",
    "ядро открыто!",
    "защищено бронёй",
];

impl Display for Battle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
"
УНИЧ... ТОЖИТЬ.

Ваше ХП: {}/{}

Сенсор: {}
Ядро: {} - {}
Левое крыло: {}
Правое крыло: {}
Орудие: {}

Его шанс попасть: {}%
",
            self.hero.health.current_health,
            self.hero.health.max_health,
            self.bygone.sensor.render(self.bygone.dodge_penalty()),
            self.bygone.core.render(self.bygone.dodge_penalty()),
            CORE_STATUS[self.bygone.core.health.current_health as usize],
            self.bygone.left_wing.render(self.bygone.dodge_penalty()),
            self.bygone.right_wing.render(self.bygone.dodge_penalty()),
            self.bygone.gun.render(self.bygone.dodge_penalty()),
            100 - self.bygone.accuracy_penalty(),
        )
    }
}
