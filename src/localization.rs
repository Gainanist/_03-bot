use std::collections::HashMap;

use crate::language::Language;

#[derive(Clone, Debug)]
pub struct Localization {
    pub health: String,
    pub dodge: String,
    pub attack: String,
    pub accuracy: String,

    pub core: String,
    pub sensor: String,
    pub gun: String,
    pub left_wing: String,
    pub right_wing: String,

    pub core_armored: String,
    pub core_exposed: String,
    pub core_burning: String,
    pub core_destroyed: String,

    pub title: String,
}

pub trait RenderText {
    fn render_text(&self, localization: &Localization) -> String;
}

#[derive(Clone, Debug)]
pub struct Localizations(HashMap<Language, Localization>);

impl Localizations {
    pub fn new() -> Self {
        let localization_ru = Localization {
            health: "ХП".to_string(),
            dodge: "".to_string(),
            attack: "АТК".to_string(),
            accuracy: "".to_string(),
        
            core: "ядро".to_string(),
            sensor: "сенсор".to_string(),
            gun: "орудие".to_string(),
            left_wing: "левое крыло".to_string(),
            right_wing: "правое крыло".to_string(),
        
            core_armored: "защищено бронёй".to_string(),
            core_exposed: "открыто!".to_string(),
            core_burning: "ГОРИТ!".to_string(),
            core_destroyed: "УНИЧТОЖЕНО!".to_string(),

            title: "УНИЧ... ТОЖИТЬ.".to_string(),
        };
        
        let localization_en = Localization {
            health: "HP".to_string(),
            dodge: "".to_string(),
            attack: "ATK".to_string(),
            accuracy: "".to_string(),
        
            core: "core".to_string(),
            sensor: "sensor".to_string(),
            gun: "gun".to_string(),
            left_wing: "left wing".to_string(),
            right_wing: "right wing".to_string(),
        
            core_armored: "armored".to_string(),
            core_exposed: "exposed!".to_string(),
            core_burning: "BURNING!".to_string(),
            core_destroyed: "DESTROYED!".to_string(),

            title: "DES... TROY.".to_string(),
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