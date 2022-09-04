use std::{collections::HashMap, fmt::Display, time::Duration};

use serde::{Deserialize, Serialize};

use crate::components::PlayerName;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    Ru,
    En,
}

impl Language {
    pub fn from_str(difficulty: &str) -> Option<Self> {
        match difficulty {
            "en" => Some(Language::En),
            "ru" => Some(Language::Ru),
            _ => None,
        }
    }
}

impl From<Language> for &str {
    fn from(language: Language) -> Self {
        match language {
            Language::Ru => "ru",
            Language::En => "en",
        }
    }
}

impl From<Language> for String {
    fn from(language: Language) -> Self {
        let slice: &str = language.into();
        slice.to_owned()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LocalizedLine(pub String);

impl LocalizedLine {
    pub fn insert_player_name(&self, name: &PlayerName) -> Self {
        LocalizedLine(self.0.replace("{PLAYER_NAME}", &name.0))
    }

    pub fn insert_enemy_name(&self, name: &str) -> Self {
        LocalizedLine(self.0.replace("{ENEMY_NAME}", name))
    }

    pub fn insert_bygone_part_name(&self, name: &str) -> Self {
        LocalizedLine(self.0.replace("{BYGONE03_PART_NAME}", name))
    }

    pub fn insert_duration(&self, duration: &Duration) -> Self {
        LocalizedLine(
            self.0
                .replace("{DURATION}", &duration.as_secs().to_string()),
        )
    }
}

impl Into<String> for &LocalizedLine {
    fn into(self) -> String {
        self.0.clone()
    }
}

impl From<&str> for LocalizedLine {
    fn from(line: &str) -> Self {
        Self(line.to_owned())
    }
}

impl Display for LocalizedLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Localization {
    pub attack: LocalizedLine,

    pub status_title: LocalizedLine,
    pub core_title: LocalizedLine,
    pub sensor_title: LocalizedLine,
    pub gun_title: LocalizedLine,
    pub left_wing_title: LocalizedLine,
    pub right_wing_title: LocalizedLine,

    pub core: LocalizedLine,
    pub sensor: LocalizedLine,
    pub gun: LocalizedLine,
    pub left_wing: LocalizedLine,
    pub right_wing: LocalizedLine,

    pub core_armored: LocalizedLine,
    pub core_exposed: LocalizedLine,
    pub core_burning: LocalizedLine,
    pub core_destroyed: LocalizedLine,

    pub turn_progress_title: LocalizedLine,
    pub log_title: LocalizedLine,
    pub player_join: Vec<LocalizedLine>,
    pub player_miss: Vec<LocalizedLine>,
    pub player_hit: Vec<LocalizedLine>,
    pub player_dead: Vec<LocalizedLine>,
    pub bygone03_miss: Vec<LocalizedLine>,
    pub bygone03_hit: Vec<LocalizedLine>,
    pub bygone03_dead: Vec<LocalizedLine>,

    pub title: LocalizedLine,
    pub lost: LocalizedLine,
    pub won: LocalizedLine,
    pub expired: Vec<LocalizedLine>,
    pub battle_cooldown: LocalizedLine,
    pub other_battle_ongoing: LocalizedLine,
}

pub trait RenderText {
    fn render_text(&self, localization: &Localization) -> String;
}

#[derive(Clone, Debug)]
pub struct Localizations(HashMap<Language, Localization>);

impl Localizations {
    pub fn new() -> Self {
        let localization_ru = Localization {
            attack: "АТК".into(),

            status_title: "Статус".into(),
            core_title: ":regional_indicator_c: ядро".into(),
            sensor_title: ":regional_indicator_s: сенсор".into(),
            gun_title: ":regional_indicator_g: орудие".into(),
            left_wing_title: ":regional_indicator_l: левое крыло".into(),
            right_wing_title: ":regional_indicator_r: правое крыло".into(),

            core: "ядро".into(),
            sensor: "сенсор".into(),
            gun: "орудие".into(),
            left_wing: "левое крыло".into(),
            right_wing: "правое крыло".into(),

            core_armored: "*защищено бронёй*".into(),
            core_exposed: "*открыто!*".into(),
            core_burning: "*ГОРИТ!*".into(),
            core_destroyed: "*УНИЧТОЖЕНО!*".into(),

            turn_progress_title: "Время хода".into(),
            log_title: "Лог битвы".into(),
            player_join: vec![
                "*{PLAYER_NAME}* рвётся в бой".into(),
                "*{PLAYER_NAME}* смастерил(а) себе рогатку".into(),
                "*{PLAYER_NAME}* всего лишь пытался(ась) выйти из Ройса".into(),
            ],
            player_miss: vec![
                "*{PLAYER_NAME}* промахивается".into(),
                "*{PLAYER_NAME}* использует стиль пьяного мастера".into(),
                "*{PLAYER_NAME}* отвлекается на силуэт в окне".into(),
            ],
            player_hit: vec![
                "*{PLAYER_NAME}* попадает в *{BYGONE03_PART_NAME}*".into(),
                "*{PLAYER_NAME}* крушит *{BYGONE03_PART_NAME}* из своей маленькой катапульты".into(),
                "*{PLAYER_NAME}* подбивает *{BYGONE03_PART_NAME}*".into(),
            ],
            player_dead: vec![
                "*{PLAYER_NAME}* отправляется в отключку".into(),
                "*{PLAYER_NAME}* идёт решать диалоговый паззл".into(),
                "*{PLAYER_NAME}* получает разрыв попы массивными резиновыми шарами".into(),
            ],
            bygone03_miss: vec![
                "*{ENEMY_NAME}* промахивается".into(),
                "У *{ENEMY_NAME}* в глазах двоится".into(),
                "*{ENEMY_NAME}* вспоминает молодость".into(),
            ],
            bygone03_hit: vec![
                "*{ENEMY_NAME}* попадает в *{PLAYER_NAME}*".into(),
                "*{ENEMY_NAME}* ласкает грудь резиновыми пулями, а *{PLAYER_NAME}* и не против".into(),
                "*{ENEMY_NAME}* предлагает бесплатный массаж, *{PLAYER_NAME}* спешит записаться".into(),
            ],
            bygone03_dead: vec![
                "Человек торжествует над машиной!".into(),
            ],

            title: "УНИЧ...ТОЖИТЬ.".into(),
            lost: "*Так темно… Я что, умер? Здесь так спокойно.*".into(),
            won: "*Человек торжествует над машиной!*".into(),
            expired: vec![
                "*Ну все, хватит. Я больше не могу на это смотреть. Дайте мне пару минут, и я разберусь с этой штукой.*".into(),
                "*Сердечко. Сердечко. Румяная кошачья мордочка. Сердечко*".into(),
            ],
            battle_cooldown: "*_03 ремонтирует себя, будет готов через {DURATION} сек*".into(),
            other_battle_ongoing: "*_03 занят: кто-то уже пытается выйти из Ройса!*".into()
        };

        let localization_en = Localization {
            attack: "ATK".into(),

            status_title: "Status".into(),
            core_title: ":regional_indicator_c:ore".into(),
            sensor_title: ":regional_indicator_s:ensor".into(),
            gun_title: ":regional_indicator_g:un".into(),
            left_wing_title: ":regional_indicator_l:eft wing".into(),
            right_wing_title: ":regional_indicator_r:ight wing".into(),

            core: "core".into(),
            sensor: "sensor".into(),
            gun: "gun".into(),
            left_wing: "left wing".into(),
            right_wing: "right wing".into(),

            core_armored: "*armored*".into(),
            core_exposed: "*exposed!*".into(),
            core_burning: "*BURNING!*".into(),
            core_destroyed: "*DESTROYED!*".into(),

            turn_progress_title: "Turn timer".into(),
            log_title: "Battle log".into(),
            player_join: vec![
                "*{PLAYER_NAME}* joins the fray".into(),
                "*{PLAYER_NAME}* have made themselves a slingshot".into(),
                "*{PLAYER_NAME}* was just trying to leave Royce".into(),
            ],
            player_miss: vec![
                "*{PLAYER_NAME}* misses".into(),
                "*{PLAYER_NAME}* fights in a drunken style".into(),
                "*{PLAYER_NAME}* gets distracted by a silhouette in the window".into(),
            ],
            player_hit: vec![
                "*{PLAYER_NAME}* hits the *{BYGONE03_PART_NAME}*".into(),
                "*{PLAYER_NAME}* smashes the *{BYGONE03_PART_NAME}* with their small catapult".into(),
                "*{PLAYER_NAME}* damages the *{BYGONE03_PART_NAME}*".into(),
            ],
            player_dead: vec![
                "*{PLAYER_NAME}* is knocked out".into(),
                "*{PLAYER_NAME}* proceeds to solve the dialogue puzzle".into(),
                "*{PLAYER_NAME}* gets their tushy ruined by the massive rubber balls".into(),
            ],
            bygone03_miss: vec![
                "*{ENEMY_NAME}* misses".into(),
                "*{ENEMY_NAME}* is seeing double".into(),
                "*{ENEMY_NAME}* is reminiscing the old days".into(),
            ],
            bygone03_hit: vec![
                "*{ENEMY_NAME}* hits *{PLAYER_NAME}*".into(),
                "*{ENEMY_NAME}* gently punches *{PLAYER_NAME}* in the chest with a rubber bullet".into(),
                "*{ENEMY_NAME}* offers *{PLAYER_NAME}* a free massage".into(),
            ],
            bygone03_dead: vec![
                "Man triumphs over machine!".into(),
            ],

            title: "DES...TROY.".into(),
            lost: "*This darkness… Am I… dead? It’s so peaceful.*".into(),
            won: "*Man triumphs over machine!*".into(),
            expired: vec![
                "*That’s it. I can’t watch this anymore. Just give me a moment, and I’ll deal with this thing.*".into(),
                "*Heart. Heart. Blushing cat face. Heart.*".into(),
            ],
            battle_cooldown: "*_03 is repairing itself, it will be ready in {DURATION}s*".into(),
            other_battle_ongoing: "*_03 is busy: somebody is already trying to leave Royce!*".into()
        };

        let mut localizations = HashMap::with_capacity(2);
        localizations.insert(Language::Ru, localization_ru);
        localizations.insert(Language::En, localization_en);

        Localizations(localizations)
    }

    pub fn get(&self, language: Language) -> &Localization {
        self.0
            .get(&language)
            .unwrap_or(self.0.get(&Language::En).unwrap())
    }
}
