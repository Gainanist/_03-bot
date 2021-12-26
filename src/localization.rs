use std::collections::HashMap;

use crate::language::Language;

#[derive(Clone, Debug)]
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

    pub intro: String,
    pub battle_decline: String,
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

            intro: "*Враг близко...*".to_string(),
            battle_decline: "Но никто не пришёл.".to_string(),
            title: "УНИЧ... ТОЖИТЬ.".to_string(),
            lost: "Так темно… Я что, умер? Здесь так спокойно.".to_string(),
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

            intro: "*A new threat is approaching...*".to_string(),
            battle_decline: "But nobody came.".to_string(),
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