use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::components::BygonePart;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    Ru,
    En,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Localization {
    pub attack: String,

    pub status_title: String,
    pub core_title: String,
    pub sensor_title: String,
    pub gun_title: String,
    pub left_wing_title: String,
    pub right_wing_title: String,

    pub core: String,
    pub core_armored: String,
    pub core_exposed: String,
    pub core_burning: String,
    pub core_destroyed: String,

    pub log_title: String,
    pub player_join: Vec<String>,
    pub player_miss: Vec<String>,
    pub player_hit: Vec<String>,
    pub player_dead: Vec<String>,
    pub bygone03_miss: Vec<String>,
    pub bygone03_hit: Vec<String>,
    pub bygone03_dead: Vec<String>,

    pub title: String,
    pub lost: String,
    pub won: String,
}

pub trait RenderText {
    fn render_text(&self, localization: &Localization) -> String;
}

#[derive(Clone, Debug)]
pub struct Localizations(HashMap<Language, Localization>);

impl Localizations {
    pub fn new() -> Self {
        let localization_ru = Localization {
            attack: "АТК".to_string(),
        
            status_title: "Статус".to_string(),
            core_title: ":regional_indicator_c: ядро".to_string(),
            sensor_title: ":regional_indicator_s: сенсор".to_string(),
            gun_title: ":regional_indicator_g: орудие".to_string(),
            left_wing_title: ":regional_indicator_l: левое крыло".to_string(),
            right_wing_title: ":regional_indicator_r: правое крыло".to_string(),

            core: "ядро".to_string(),
            core_armored: "защищено бронёй".to_string(),
            core_exposed: "открыто!".to_string(),
            core_burning: "ГОРИТ!".to_string(),
            core_destroyed: "УНИЧТОЖЕНО!".to_string(),

            log_title: "Лог битвы".to_string(),
            player_join: vec![
                "{PLAYER_NAME} рвётся в бой".to_string(),
                "{PLAYER_NAME} смастерил(а) себе рогатку".to_string(),
                "{PLAYER_NAME} всего лишь пытался(ась) выйти из Ройса".to_string(),
            ],
            player_miss: vec![
                "{PLAYER_NAME} промахивается".to_string(),
                "{PLAYER_NAME} использует стиль пьяного мастера".to_string(),
                "{PLAYER_NAME} отвлекается на силуэт в окне".to_string(),
            ],
            player_hit: vec![
                "{PLAYER_NAME} попадает в {BYGONE03_PART_NAME}".to_string(),
                "{PLAYER_NAME} крушит {BYGONE03_PART_NAME} из своей маленькой катапульты".to_string(),
                "{PLAYER_NAME} подбивает {BYGONE03_PART_NAME}".to_string(),
            ],
            player_dead: vec![
                "{PLAYER_NAME} отправляется в отключку".to_string(),
                "{PLAYER_NAME} идёт решать диалоговый паззл".to_string(),
                "{PLAYER_NAME} получает разрыв попы массивными резиновыми шарами".to_string(),
            ],
            bygone03_miss: vec![
                "{ENEMY_NAME} промахивается".to_string(),
                "У {ENEMY_NAME} в глазах двоится".to_string(),
                "{ENEMY_NAME} вспоминает молодость".to_string(),
            ],
            bygone03_hit: vec![
                "{ENEMY_NAME} попадает в {PLAYER_NAME}".to_string(),
                "{ENEMY_NAME} ласкает грудь резиновыми пулями, а {PLAYER_NAME} и не против".to_string(),
                "{ENEMY_NAME} предлагает бесплатный массаж, {PLAYER_NAME} спешит записаться".to_string(),
            ],
            bygone03_dead: vec![
                "Человек торжествует над машиной!".to_string(),
            ],

            title: "УНИЧ... ТОЖИТЬ.".to_string(),
            lost: "*Так темно… Я что, умер? Здесь так спокойно.*".to_string(),
            won: "*Человек торжествует над машиной!*".to_string(),
        };
        
        let localization_en = Localization {
            attack: "ATK".to_string(),

            status_title: "Status".to_string(),
            core_title: ":regional_indicator_c:ore".to_string(),
            sensor_title: ":regional_indicator_s:ensor".to_string(),
            gun_title: ":regional_indicator_g:un".to_string(),
            left_wing_title: ":regional_indicator_l:eft wing".to_string(),
            right_wing_title: ":regional_indicator_r:ight wing".to_string(),

            core: "core".to_string(),
            core_armored: "armored".to_string(),
            core_exposed: "exposed!".to_string(),
            core_burning: "BURNING!".to_string(),
            core_destroyed: "DESTROYED!".to_string(),

            log_title: "Battle log".to_string(),
            player_join: vec![
                "{PLAYER_NAME} joins the fray".to_string(),
                "{PLAYER_NAME} have made themselves a slingshot".to_string(),
                "{PLAYER_NAME} was just trying to leave Royce".to_string(),
            ],
            player_miss: vec![
                "{PLAYER_NAME} misses".to_string(),
                "{PLAYER_NAME} fights with a drunken style".to_string(),
                "{PLAYER_NAME} gets distracted by a silhouette in the window".to_string(),
            ],
            player_hit: vec![
                "{PLAYER_NAME} hits the {BYGONE03_PART_NAME}".to_string(),
                "{PLAYER_NAME} smashes the {BYGONE03_PART_NAME} using their small catapult".to_string(),
                "{PLAYER_NAME} damages the {BYGONE03_PART_NAME}".to_string(),
            ],
            player_dead: vec![
                "{PLAYER_NAME} is knocked out".to_string(),
                "{PLAYER_NAME} proceeds to solve the dialogue puzzle".to_string(),
                "{PLAYER_NAME} gets their tushy ruined by the massive rubber balls".to_string(),
            ],
            bygone03_miss: vec![
                "{ENEMY_NAME} misses".to_string(),
                "{ENEMY_NAME} is seeing double".to_string(),
                "{ENEMY_NAME} is reminiscing the old days".to_string(),
            ],
            bygone03_hit: vec![
                "{ENEMY_NAME} hits {PLAYER_NAME}".to_string(),
                "{ENEMY_NAME} gently punches {PLAYER_NAME} in the chest with a rubber bullet".to_string(),
                "{ENEMY_NAME} offers {PLAYER_NAME} a free massage".to_string(),
            ],
            bygone03_dead: vec![
                "Man triumphs over machine!".to_string(),
            ],

            title: "A wild _03 appears!".to_string(),
            lost: "*This darkness… Am I… dead? It’s so peaceful.*".to_string(),
            won: "*Man triumphs over machine!*".to_string(),
        };

        let mut localizations = HashMap::with_capacity(2);
        localizations.insert(Language::Ru, localization_ru);
        localizations.insert(Language::En, localization_en);
        
        Localizations(localizations)
    }

    pub fn get(&self, language: Language) -> &Localization {
        self.0.get(&language).unwrap_or(
            self.0.get(&Language::En).unwrap()
        )
    }
}