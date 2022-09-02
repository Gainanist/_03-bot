use std::{collections::HashMap, fmt::Display, time::Duration};

use serde::{Deserialize, Serialize};

use crate::components::{PlayerName};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    Ru,
    En,
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
        LocalizedLine(self.0.replace("{DURATION}", &duration.as_secs().to_string()))
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
            attack: LocalizedLine("АТК".to_string()),

            status_title: LocalizedLine("Статус".to_string()),
            core_title: LocalizedLine(":regional_indicator_c: ядро".to_string()),
            sensor_title: LocalizedLine(":regional_indicator_s: сенсор".to_string()),
            gun_title: LocalizedLine(":regional_indicator_g: орудие".to_string()),
            left_wing_title: LocalizedLine(":regional_indicator_l: левое крыло".to_string()),
            right_wing_title: LocalizedLine(":regional_indicator_r: правое крыло".to_string()),

            core: LocalizedLine("ядро".to_string()),
            sensor: LocalizedLine("сенсор".to_string()),
            gun: LocalizedLine("орудие".to_string()),
            left_wing: LocalizedLine("левое крыло".to_string()),
            right_wing: LocalizedLine("правое крыло".to_string()),

            core_armored: LocalizedLine("*защищено бронёй*".to_string()),
            core_exposed: LocalizedLine("*открыто!*".to_string()),
            core_burning: LocalizedLine("*ГОРИТ!*".to_string()),
            core_destroyed: LocalizedLine("*УНИЧТОЖЕНО!*".to_string()),

            turn_progress_title: LocalizedLine("Время хода".to_owned()),
            log_title: LocalizedLine("Лог битвы".to_string()),
            player_join: vec![
                LocalizedLine("*{PLAYER_NAME}* рвётся в бой".to_string()),
                LocalizedLine("*{PLAYER_NAME}* смастерил(а) себе рогатку".to_string()),
                LocalizedLine("*{PLAYER_NAME}* всего лишь пытался(ась) выйти из Ройса".to_string()),
            ],
            player_miss: vec![
                LocalizedLine("*{PLAYER_NAME}* промахивается".to_string()),
                LocalizedLine("*{PLAYER_NAME}* использует стиль пьяного мастера".to_string()),
                LocalizedLine("*{PLAYER_NAME}* отвлекается на силуэт в окне".to_string()),
            ],
            player_hit: vec![
                LocalizedLine("*{PLAYER_NAME}* попадает в *{BYGONE03_PART_NAME}*".to_string()),
                LocalizedLine("*{PLAYER_NAME}* крушит *{BYGONE03_PART_NAME}* из своей маленькой катапульты".to_string()),
                LocalizedLine("*{PLAYER_NAME}* подбивает *{BYGONE03_PART_NAME}*".to_string()),
            ],
            player_dead: vec![
                LocalizedLine("*{PLAYER_NAME}* отправляется в отключку".to_string()),
                LocalizedLine("*{PLAYER_NAME}* идёт решать диалоговый паззл".to_string()),
                LocalizedLine("*{PLAYER_NAME}* получает разрыв попы массивными резиновыми шарами".to_string()),
            ],
            bygone03_miss: vec![
                LocalizedLine("*{ENEMY_NAME}* промахивается".to_string()),
                LocalizedLine("У *{ENEMY_NAME}* в глазах двоится".to_string()),
                LocalizedLine("*{ENEMY_NAME}* вспоминает молодость".to_string()),
            ],
            bygone03_hit: vec![
                LocalizedLine("*{ENEMY_NAME}* попадает в *{PLAYER_NAME}*".to_string()),
                LocalizedLine("*{ENEMY_NAME}* ласкает грудь резиновыми пулями, а *{PLAYER_NAME}* и не против".to_string()),
                LocalizedLine("*{ENEMY_NAME}* предлагает бесплатный массаж, *{PLAYER_NAME}* спешит записаться".to_string()),
            ],
            bygone03_dead: vec![
                LocalizedLine("Человек торжествует над машиной!".to_string()),
            ],

            title: LocalizedLine("УНИЧ... ТОЖИТЬ.".to_string()),
            lost: LocalizedLine("*Так темно… Я что, умер? Здесь так спокойно.*".to_string()),
            won: LocalizedLine("*Человек торжествует над машиной!*".to_string()),
            battle_cooldown: "*_03 ремонтирует себя, будет готов через {DURATION} сек*".into(),
            other_battle_ongoing: "_03 занят: кто-то уже пытается выйти из Ройса!".into()
        };

        let localization_en = Localization {
            attack: LocalizedLine("ATK".to_string()),

            status_title: LocalizedLine("Status".to_string()),
            core_title: LocalizedLine(":regional_indicator_c:ore".to_string()),
            sensor_title: LocalizedLine(":regional_indicator_s:ensor".to_string()),
            gun_title: LocalizedLine(":regional_indicator_g:un".to_string()),
            left_wing_title: LocalizedLine(":regional_indicator_l:eft wing".to_string()),
            right_wing_title: LocalizedLine(":regional_indicator_r:ight wing".to_string()),

            core: LocalizedLine("core".to_string()),
            sensor: LocalizedLine("sensor".to_string()),
            gun: LocalizedLine("gun".to_string()),
            left_wing: LocalizedLine("left wing".to_string()),
            right_wing: LocalizedLine("right wing".to_string()),

            core_armored: LocalizedLine("*armored*".to_string()),
            core_exposed: LocalizedLine("*exposed!*".to_string()),
            core_burning: LocalizedLine("*BURNING!*".to_string()),
            core_destroyed: LocalizedLine("*DESTROYED!*".to_string()),

            turn_progress_title: LocalizedLine("Turn timer".to_owned()),
            log_title: LocalizedLine("Battle log".to_string()),
            player_join: vec![
                LocalizedLine("*{PLAYER_NAME}* joins the fray".to_string()),
                LocalizedLine("*{PLAYER_NAME}* have made themselves a slingshot".to_string()),
                LocalizedLine("*{PLAYER_NAME}* was just trying to leave Royce".to_string()),
            ],
            player_miss: vec![
                LocalizedLine("*{PLAYER_NAME}* misses".to_string()),
                LocalizedLine("*{PLAYER_NAME}* fights in a drunken style".to_string()),
                LocalizedLine("*{PLAYER_NAME}* gets distracted by a silhouette in the window".to_string()),
            ],
            player_hit: vec![
                LocalizedLine("*{PLAYER_NAME}* hits the *{BYGONE03_PART_NAME}*".to_string()),
                LocalizedLine("*{PLAYER_NAME}* smashes the *{BYGONE03_PART_NAME}* with their small catapult".to_string()),
                LocalizedLine("*{PLAYER_NAME}* damages the *{BYGONE03_PART_NAME}*".to_string()),
            ],
            player_dead: vec![
                LocalizedLine("*{PLAYER_NAME}* is knocked out".to_string()),
                LocalizedLine("*{PLAYER_NAME}* proceeds to solve the dialogue puzzle".to_string()),
                LocalizedLine("*{PLAYER_NAME}* gets their tushy ruined by the massive rubber balls".to_string()),
            ],
            bygone03_miss: vec![
                LocalizedLine("*{ENEMY_NAME}* misses".to_string()),
                LocalizedLine("*{ENEMY_NAME}* is seeing double".to_string()),
                LocalizedLine("*{ENEMY_NAME}* is reminiscing the old days".to_string()),
            ],
            bygone03_hit: vec![
                LocalizedLine("*{ENEMY_NAME}* hits *{PLAYER_NAME}*".to_string()),
                LocalizedLine("*{ENEMY_NAME}* gently punches *{PLAYER_NAME}* in the chest with a rubber bullet".to_string()),
                LocalizedLine("*{ENEMY_NAME}* offers *{PLAYER_NAME}* a free massage".to_string()),
            ],
            bygone03_dead: vec![
                LocalizedLine("Man triumphs over machine!".to_string()),
            ],

            title: LocalizedLine("A wild _03 appears!".to_string()),
            lost: LocalizedLine("*This darkness… Am I… dead? It’s so peaceful.*".to_string()),
            won: LocalizedLine("*Man triumphs over machine!*".to_string()),
            battle_cooldown: "*_03 is repairing itself, it will be ready in {DURATION}s*".into(),
            other_battle_ongoing: "_03 is busy: somebody is already trying to leave Royce!".into()
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
