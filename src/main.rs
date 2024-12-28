mod config;
mod commands;

use config::Config;
use poise::serenity_prelude::{self as serenity, CreateEmbed};
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::sync::{LazyLock, Mutex};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, (), Error>;

const config: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::create()));
const draft_state: LazyLock<Mutex<DraftState>> = LazyLock::new(|| Mutex::new(DraftState::new()));

pub struct DraftState {
    players: Vec<Player>,
    teams: HashMap<Captain, Vec<Player>>,
    draft_started: bool,
    current_round: u32,
    round_captain: Option<Captain>,
    nominated_player: Option<Player>,
    bid_placed: bool,
    starting_bid: u32,
    current_bid: (u32, Option<Captain>),
    time: u32,
}

impl DraftState {
    pub fn new() -> Self {
        Self {
            players: vec![],
            teams: HashMap::new(),
            draft_started: false,
            current_round: 0,
            nominated_player: None,
            round_captain: None,
            bid_placed: false,
            starting_bid: 0,
            current_bid: (0, None),
            time: 0,
        }
    }
}

pub struct Captain {
    name: String,
    balance: u32,
    discord_id: u64,
    illegal_alien_count: u32, // thank god it's not toronto
}

impl Captain {
    pub fn new(name: String, balance: u32, discord_id: u64) -> Self {
        Self {
            name,
            balance,
            discord_id,
            illegal_alien_count: 0,
        }
    }

    pub fn get_max_bid(&self) -> u32 {
        let cfg = config.lock().unwrap();
        let slots_left = cfg.team_size - self.players.len() as u32;
        return self.balance - slots_left * cfg.min_bid;
    }

    pub async fn add_player(&self, player: &Player) {
        if player.is_illegal {
            self.illegal_alien_count += 1;
        }

        p1.team = Some();
        c1.players.push(player);
    }

    pub async fn to_string(&self) -> String {
        let mut player_str = String::from("");
        if self.players.len() == 0 {
            player_str = String::from("None");
        }
        for player in self.players.iter() {
            let p1 = player.lock().await;
            player_str = player_str + &p1.name.clone();
        }
        return format!(
            "Name: {}\nBalance: {}\n Players: {}",
            self.name, self.balance, player_str,
        );
    }
}

pub struct Player {
    name: String,
    is_illegal: bool,
    recent_wn8: u32,
    team_id: Option<u64>,
}

impl Player {
    pub fn new(name: String, is_illegal: bool) -> Self {
        Self {
            name,
            is_illegal,
            recent_wn8: 0,
            team_id: None,
        }
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub fn import_players() -> Vec<Player> {
    fs::read_to_string("/home/zray/code/auction2025/src/players.csv")
        .unwrap()
        .split('\n')
        .map(|line| line.split_once(',').unwrap_or((line, "")))
        .map(|(name, legio)| Player::new(name.to_string(), legio == "legio"))
        .collect()
}

pub async fn autocomplete_player<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    draft_state
        .lock().unwrap()
        .players.iter()
        .filter(|player| player.name.to_lowercase().starts_with(&partial.to_lowercase()) && player.team_id.is_none())
        .map(|player| player.name.clone())
        .collect::<Vec<String>>()
        .into_iter()
}

pub fn get_wn8_color(wn8: u32) -> i32 {
    match wn8 {
        0 => 0x808080,
        1..=300 => 0x930D0D,
        301..=450 => 0xCD3333,
        451..=650 => 0xCC7A00,
        651..=900 => 0xCCB800,
        901..=1200 => 0x849B24,
        1201..=1600 => 0x4D7326,
        1601..=2000 => 0x4099BF,
        2001..=2450 => 0x3972C6,
        2451..=2900 => 0x6844d4,
        2901..=3400 => 0x522b99,
        3401..=4000 => 0x411d73,
        4001..=4700 => 0x310d59,
        4701..=u32::MAX => 0x24073d,
    }
}
pub async fn make_final_draft_embed() -> CreateEmbed {
    CreateEmbed::default().title("Draft").fields(
        draft_state.lock().unwrap().teams.iter().map(|(captain, players)| 
            (
                captain.name.clone(), 
                players.iter().map(|player| player.name.clone()).collect::<Vec<String>>().join("\n"),
                true
            )
        )
    )
}

pub async fn generate_draft_embed() -> CreateEmbed {

    let ds = draft_state;
    let ds = ds.lock().unwrap();

    CreateEmbed::default().title("Draft").fields(
        [
            (
                "Round Info",
                match &ds.round_captain {
                    Some(captain) => format!(
                        "Round: `{}`\nCaptain: '{}'\nTime Left: {}",
                        ds.current_round, captain.name, ds.time,
                    ),
                    None => format!(
                        "Round: `{}`\nCaptain: '{}'",
                        ds.current_round, "None",
                    )
                },
                true
            ), (
                "Player info",
                match &ds.nominated_player {
                    Some(player) => format!("Name: `{}`\nRecent wn8: `{}`", player.name, player.recent_wn8),
                    None => format!(
                        "Name: `{}`\nRecent wn8: `{}`\nInfo: '{}'",
                        "None", "0", "None",
                    )
                },
                true
            ), (
                "Bid Info",
                match &ds.current_bid.1 {
                    Some(captain) => format!(
                        "Starting Bid: `{}`\nCurrent Winner: `{}`\nCurrent Bid: `{}`",
                        ds.starting_bid, captain.name, ds.current_bid.0,
                    ),
                    None => format!(
                        "Starting Bid: `{}`\nCurrent Winner: `{}`\nCurrent Bid: `{}`",
                        ds.starting_bid, "None", ds.current_bid.0,
                    )
                },
                true
            )
        ]
    )
}


#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("Missing Token");
    let intents = serenity::GatewayIntents::non_privileged();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::display_players(),
                commands::start_draft(),
                commands::add_captain(),
                commands::pick(),
                commands::bid(),
                commands::display_captains(),
                commands::set_config(),
                commands::display_teams(),
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    1318516792821420062.try_into().unwrap(),
                )
                .await?;
                Ok(())
            })
        })
        .build();
    
    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
