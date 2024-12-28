use poise::serenity_prelude::{self as serenity, CreateEmbed, EditMessage};
use poise::CreateReply;
use tokio::sync::Mutex;
use std::sync::Arc;
use rand::thread_rng;
use rand::seq::SliceRandom;
use tokio::time;
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a,UserData,Error>;


pub struct UserData {
    captains: Mutex<Vec<Arc<Mutex<Captain>>>>,
    players: Mutex<Vec<Arc<Mutex<Player>>>>,
    config: Mutex<Config>,
    draft_state: Mutex<DraftState>,
}


pub struct DraftState {
    draft_started: bool,
    current_round: u32,
    nominated_player: Option<Arc<Mutex<Player>>>,
    round_captain: Option<Arc<Mutex<Captain>>>,
    bid_placed: bool,
    starting_bid: u32,
    current_bid: u32,
    current_winner: Option<Arc<Mutex<Captain>>>,
    time: u32,
}

impl DraftState {
    pub fn new() -> Self {
        Self {
            draft_started: false,
            current_round: 0,
            nominated_player: None,
            round_captain: None,
            bid_placed: false,
            starting_bid: 0,
            current_bid: 0,
            current_winner: None,
            time: 0,
        }
    }
}



pub struct Config {
    min_bid: u32,
    starting_balance: u32,
    team_size: u32,
    round_time: u32,
    bid_add_time: u32,
    legio_limit: u32,
}
impl Config {
    pub fn default() -> Self {
        Self {
            min_bid: 10,
            starting_balance: 200,
            team_size: 8,
            round_time: 20,
            bid_add_time: 5,
            legio_limit: 2,
        }
    }
    pub fn to_string(&self) -> String {
        let mut config_str = String::new();
        config_str += &format!("Min Bid: {}", self.min_bid);
        config_str.push('\n');
        config_str += &format!("Starting Balance: {}", self.starting_balance);
        config_str.push('\n');
        config_str += &format!("Team Size: {}", self.team_size);
        config_str.push('\n');
        config_str += &format!("Round Time: {}", self.round_time);
        config_str.push('\n');
        config_str += &format!("Bid add team: {}", self.bid_add_time);
        config_str.push('\n');
        config_str += &format!("Legio Limit: {}", self.legio_limit);
        config_str.push('\n');
        return config_str;

    }
}
pub async fn add_player_to_captain(captain:Arc<Mutex<Captain>>, player: Arc<Mutex<Player>>) {
    let mut p1 = player.lock().await;
    let mut c1 = captain.lock().await;
    if p1.is_legio {
        c1.legio_count +=1;
    }
    p1.picked = true;
    p1.team = Some(captain.clone());
    drop(p1);
    c1.players.push(player);
}

pub struct Captain {
    discord_id: u64,
    name: String,
    players: Vec<Arc<Mutex<Player>>>,
    balance: u32,
    legio_count: u32,
}
impl Captain {
    pub fn new(id: u64, name: String, bal: u32) -> Self {
        Self {
            discord_id: id,
            name,
            players: Vec::new(),
            balance: bal,
            legio_count: 0,
        }
    }
    
    pub fn get_max_bid(&self, config: &Config) -> u32 {
        let slots_left = config.team_size - self.players.len() as u32;
        return self.balance - slots_left*config.min_bid;

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
        return format!("Name: {}\nBalance: {}\n Players: {}",
            self.name,
            self.balance,
            player_str,
            )
    }

}

pub struct Player {
    name: String,
    is_legio: bool,
    recent_wn8: u32,
    team: Option<Arc<Mutex<Captain>>>,
    picked: bool,
}


impl Player {
    pub fn new(name: String, is_legio: bool) -> Self {
        Self {
            name,
            is_legio,
            recent_wn8: 0,
            team: None,
            picked: false,
        }
    }
    pub fn to_string(&self) -> String {
        return self.name.clone();
    }
}

/// Displays Captains, Use Display Teams instead
#[poise::command(slash_command)]
async fn display_captains(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let mut captain_str = String::from("Captains: \n");
    let captains = ctx.data().captains.lock().await;
    for captain in captains.iter() {
        captain_str += &(captain.lock().await.name);
        captain_str += "\n";
    }
    let _ = ctx.say(captain_str).await;
    Ok(())
}

#[poise::command(slash_command)]
async fn config(
    ctx: Context<'_>,
    #[description = "Round Time"] round_time: Option<u32>,
    #[description = "Bid Add Time"] bid_add_time: Option<u32>,
    #[description = "Minimum Bid"] min_bid: Option<u32>,
    #[description = "Starting Balance"] starting_balance: Option<u32>,
    #[description = "Team Size"] team_size: Option<u32>,
    #[description = "Legio Limit"] legio_limit: Option<u32>,

) ->Result<(), Error> {
    let mut config = ctx.data().config.lock().await;
    if let Some(rt) = round_time {
        config.round_time = rt;
    }
    if let Some(bt) = bid_add_time {
        config.bid_add_time = bt;
    }
    if let Some(mb) = min_bid {
        config.min_bid = mb;
    }
    if let Some(sb) = starting_balance {
        let draft_state = ctx.data().draft_state.lock().await;
        if draft_state.draft_started {

            let _ = ctx.send(CreateReply::default()
                .content("Cannot change starting balance after start")
                .reply(true)
                .ephemeral(true))
                .await;
            return Ok(())
        }
        config.starting_balance = sb;
    }
    if let Some(ts) = team_size {
        let draft_state = ctx.data().draft_state.lock().await;
        if draft_state.draft_started {
            let _ = ctx.send(CreateReply::default()
                .content("Cannot change team size after start")
                .reply(true)
                .ephemeral(true))
                .await;
            return Ok(())
        }
        config.team_size = ts;
    }
    if let Some(ll) = legio_limit {
        let draft_state = ctx.data().draft_state.lock().await;
        if draft_state.draft_started {
            let _ = ctx.say(
                "This might not function properly after draft start. Restart draft if early."
                );
        }
        config.legio_limit = ll;
    }
    let _ = ctx.say(config.to_string()).await;
    
    return Ok(())

}

#[poise::command(slash_command)]
async fn display_teams(
    ctx: Context<'_>,
) -> Result<(), Error> {
    
    let captains = ctx.data().captains.lock().await;
    let embed = make_final_draft_embed(captains.to_vec()).await;
    let _ = ctx.send(CreateReply::default().embed(embed)).await;
    Ok(())
}

/// Displays players (for debugging do not use) DO NOT USE
#[poise::command(slash_command)]
async fn display_players(
    ctx: Context<'_>,
) -> Result<(), Error> {
    
    let players = ctx.data().players.lock().await;
    for player in players.iter() {
        let p = player.lock().await;
        ctx.say(&p.name).await?;
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn add_captain(
    ctx: Context<'_>,
    #[description = "Select User"] user: serenity::User,
    #[description = "Captain Name"] name: String,
) -> Result<(), Error> {
    let u = user;
    let config = ctx.data().config.lock().await; 
    let captain = Captain::new(
        u64::from(u.id),  
        name.clone(),
        config.starting_balance,
    );
    let mut captains = ctx.data().captains.lock().await;
    for captain in captains.to_vec() {
        if captain.lock().await.discord_id == u64::from(u.id) {

            let _ = ctx.send(CreateReply::default()
                .content(format!("Already Added this captain"))
                .reply(true)
                .ephemeral(false))
                .await;
            return Ok(())
        }
    }
    captains.push(Arc::new(Mutex::new(captain)));
    let _ = ctx.send(CreateReply::default()
        .content(format!("Added captain {}", name))
        .reply(true)
        .ephemeral(false))
        .await;
    Ok(())

}


#[poise::command(slash_command)]
async fn pick(
    ctx: Context<'_>,
    #[description = "Select a Player"]
    #[autocomplete = "autocomplete_player"]
    player: String,
    #[description = "Starting Bid"]
    starting_bid: Option<u32>,
) -> Result<(), Error> {
    let mut draft_state = ctx.data().draft_state.lock().await;
    let current_captain = &draft_state.round_captain;
    let max_bid;
    let config = ctx.data().config.lock().await;
    if draft_state.time != config.round_time {
        let _ = ctx.send(CreateReply::default()
            .content("Only pick before round starts")
            .reply(true)
            .ephemeral(true))
            .await;
        return Ok(())
    }
    let mut final_bid = config.min_bid;
    drop(config);
    match current_captain {
        Some(captain) => {
            let c1 = captain.clone();
            let c = c1.lock().await;
            if ctx.author().id != c.discord_id {
                let _ = ctx.send(CreateReply::default()
                    .content("You not da captain blud")
                    .reply(true)
                    .ephemeral(true))
                    .await;
                return Ok(());
            }
            let config = ctx.data().config.lock().await;
            max_bid = c.get_max_bid(&config);
            draft_state.current_winner = Some(captain.clone());
        }
        None => {
            let _ = ctx.send(CreateReply::default()
                .content("No Draft Running")
                .reply(true)
                .ephemeral(true))
                .await;
            return Ok(())
        }

    }

    let message = ctx.reply("Pick Processed").await?;
    message.delete(ctx).await?;
    drop(draft_state);
    let players = ctx.data().players.lock().await;
    for p_lock in players.iter() {
        let p = p_lock.lock().await;
        if p.name.eq(&player) && !p.picked {
            let mut draft_state = ctx.data().draft_state.lock().await;
            if let Some(bid) = starting_bid {
                if bid > max_bid {
                    let _ = ctx.reply("no");
                    let _ = ctx.send(CreateReply::default()
                        .content(format!(
                                "Not enough funds, Your max bid is:{}"
                                ,max_bid))
                        .reply(true)
                        .ephemeral(true))
                        .await;
                    return Ok(());
                }
                final_bid = bid;
                
            }
                draft_state.nominated_player = Some(p_lock.clone());
                draft_state.starting_bid = final_bid;
                draft_state.current_bid = final_bid;
            
        }
    }



    Ok(())
}

pub fn import_players() -> Vec<Arc<Mutex<Player>>> {
    let mut rdr = csv::Reader::from_path(
        "/home/zray/code/auction2025/src/players.csv"
    ).unwrap();
    let mut players: Vec<Arc<Mutex<Player>>> = Vec::new();
    for result in rdr.records() {
        let record = result.unwrap();
        let name = record.get(0).unwrap();
        let player = Player::new(
            name.to_string(),
            false,
        );
        players.push(Arc::new(Mutex::new(player)));

    }
    return players;
}

pub async fn autocomplete_player<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    let players = ctx.data().players.lock().await;
    let mut player_strs = Vec::new();

    for player in players.iter() {
        let p1 = player.lock().await;
        if p1.name.to_lowercase().starts_with(&partial.to_lowercase()) && !p1.picked {
            player_strs.push(p1.name.clone());
        }

    }
    return player_strs.into_iter();

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
pub async fn make_final_draft_embed(captains: Vec<Arc<Mutex<Captain>>>) -> CreateEmbed {
    let mut embed = CreateEmbed::default().title("Draft");
    for captain in captains {
        let c1 = captain.lock().await;
        let mut player_str = String::from("");
        for player in &c1.players {
            let p1 = player.lock().await;
            player_str.push_str(&p1.name);
            player_str.push_str("\n");
        }
        embed = embed.field(
            c1.name.clone(),
            player_str,
            true
        );
    }
    return embed;
}

pub async fn generate_draft_embed(draft_state: &DraftState) -> CreateEmbed {
    let mut embed = CreateEmbed::default().title("Draft");
    if let Some(captain) = &draft_state.round_captain {
        let name = &captain.lock().await.name;
        embed = embed.field(
            "Round Info",
            format!("Round: `{}`\nCaptain: '{}'\nTime Left: {}",
                draft_state.current_round,
                name,
                draft_state.time,
                ),
                true
        );
    }
    else {
        
        embed = embed.field(
            "Round Info",
            format!("Round: `{}`\nCaptain: '{}'",
                draft_state.current_round,
                "None",
                ),
                true
        );
    }
    if let Some(player) = &draft_state.nominated_player {
        let p1 = player.lock().await;
        embed = embed.field(
            "Player info",
            format!("Name: `{}`\nRecent wn8: `{}`",
                p1.name,
                p1.recent_wn8,
            ),
            true

        );
    }
    else {
        embed = embed.field(
            "Player info",
            format!("Name: `{}`\nRecent wn8: `{}`\nInfo: '{}'",
                "None",
                "0",
                "None",
            ),
            true

        );
    }
    if let Some(captain) = &draft_state.current_winner {
        let c1 = captain.lock().await;
        embed = embed.field(
            "Bid Info",
            format!("Starting Bid: `{}`\nCurrent Winner: `{}`\nCurrent Bid: `{}`",
                draft_state.starting_bid,
                c1.name,
                draft_state.current_bid,

            ),
            true
        );
    }
    else {
        embed = embed.field(
            "Bid Info",
            format!("Starting Bid: `{}`\nCurrent Winner: `{}`\nCurrent Bid: `{}`",
                draft_state.starting_bid,
                "None",
                draft_state.current_bid,

            ),
            true
        );

    }
    embed
    
}

#[poise::command(slash_command)]
async fn bid(
    ctx: Context<'_>,
    #[description = "Amount"] amount: u32,
) -> Result<(), Error> {
    //check if time left, if greater than current max increase.
    let mut draft_state = ctx.data().draft_state.lock().await;
    if draft_state.time > 0 {
        if amount > draft_state.current_bid {
            let captains = ctx.data().captains.lock().await;
            let mut captain: Option<Arc<Mutex<Captain>>> = None;
            let mut max_bid = 0;
            let mut found = false;
            for c_lock in captains.iter() {
                let c = c_lock.lock().await;
                if c.discord_id == u64::from(ctx.author().id) {
                    captain = Some(c_lock.clone());
                    let config = ctx.data().config.lock().await;
                    max_bid = c.get_max_bid(&config);
                    drop(config);
                    found = true;
                    break;
                } 
            }
            if !found {
                let _ = ctx.send(CreateReply::default()
                    .content("Not in captain list")
                    .reply(true)
                    .ephemeral(true))
                    .await;
                return Ok(())
            }
            if amount > max_bid {

                ctx.send(CreateReply::default()
                    .content(format!("Not enough funds, Your max bid is:{}",max_bid))
                    .reply(true)
                    .ephemeral(true))
                    .await?;

                return Ok(())
            }
            draft_state.current_bid = amount;
            draft_state.current_winner = Some(captain.unwrap()); 
            draft_state.bid_placed = true;
            let msg = ctx.send(CreateReply::default()
                .content(format!("You Bid ${} for {}",amount,draft_state.nominated_player.as_ref().unwrap().lock().await.name))
                .reply(true)
                .ephemeral(true))
                .await?;
            drop(draft_state);
            msg.delete(ctx).await?;
            return Ok(())
            
        }
        else {
            ctx.send(CreateReply::default()
                .content("Under current top bid")
                .reply(true)
                .ephemeral(true))
                .await?;
            return Ok(())
        }
    }
    else {
        let _ = ctx.send(CreateReply::default()
            .content("No Auction Running")
            .reply(true)
            .ephemeral(true))
            .await;
        return Ok(())

    }

}



#[poise::command(slash_command)]
async fn start_draft(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let _ = ctx.defer().await;
    let mut captains_main = ctx.data().captains.lock().await;
    captains_main.shuffle(&mut thread_rng());
    let config = ctx.data().config.lock().await;
    let team_size = config.team_size;
    let captains = captains_main.clone();
    drop(captains_main);
    let mut draft_state = ctx.data().draft_state.lock().await;
    let mut embed = generate_draft_embed(&draft_state).await;
    draft_state.draft_started = true;
    drop(draft_state);
    drop(config);
    let mut message = ctx.send(CreateReply::default().embed(embed)).await?.into_message().await?;
    let mut message2 = ctx.send(CreateReply::default().content("Draft Started")).await?.into_message().await?;
    for i in 1..team_size+1 {
        for captain in captains.iter() {
            let mut draft_state = ctx.data().draft_state.lock().await;
            let config = ctx.data().config.lock().await;
            draft_state.current_round = i;
            draft_state.nominated_player = None;
            draft_state.starting_bid = config.min_bid;
            draft_state.current_bid = config.min_bid;
            draft_state.round_captain = Some(captain.clone());
            draft_state.current_winner = None;
            draft_state.time = config.round_time;
            embed = generate_draft_embed(&draft_state).await;
            message.edit(ctx,EditMessage::default().embed(embed)).await?;
            drop(draft_state);
            drop(config);
            let mut interval = time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                let mut draft_state = ctx.data().draft_state.lock().await;
                let config = ctx.data().config.lock().await;
                if config.round_time != draft_state.time {
                    draft_state.time = config.round_time;
                    embed = generate_draft_embed(&draft_state).await;
                    message.edit(ctx,EditMessage::default().embed(embed)).await?;
                }
                if draft_state.nominated_player.is_some() {break;}
            }
            let config = ctx.data().config.lock().await;
            let draft_state = ctx.data().draft_state.lock().await;
            embed = generate_draft_embed(&draft_state).await;
            message2.edit(ctx,EditMessage::default().content(
                    format!("{} bid {} for {}", 
                        draft_state.current_winner.as_ref().unwrap().lock().await.name,
                        draft_state.current_bid,
                        draft_state.nominated_player.as_ref().unwrap().lock().await.name,

                    )
            )).await?;
            drop(draft_state);
            message.edit(ctx,EditMessage::default().embed(embed)).await?;
            let mut time_left: u32 = config.round_time;
            drop(config);
            while time_left > 0 {
                interval.tick().await;
                let mut draft_state = ctx.data().draft_state.lock().await;
                if time_left % 2 != 0 {
                    time_left +=1;
                }
                time_left = time_left-2;
                draft_state.time = time_left;
                embed = generate_draft_embed(&draft_state).await;
                message.edit(ctx,EditMessage::default().embed(embed)).await?;
                if draft_state.bid_placed == true {
                    let config = ctx.data().config.lock().await;
                    time_left = time_left + config.bid_add_time;
                    draft_state.bid_placed = false;
                    message2.edit(ctx,EditMessage::default()
                        .content(
                            format!("{} bid {} for {}", 
                                draft_state.current_winner.as_ref().unwrap().lock().await.name,
                                draft_state.current_bid,
                                draft_state.nominated_player.as_ref().unwrap().lock().await.name,

                            )
                        )
                    ).await?;
                }
            }
            let draft_state = ctx.data().draft_state.lock().await;
            let mut winner = draft_state.current_winner.as_ref().unwrap().lock().await;
            winner.balance = winner.balance - draft_state.current_bid;
            drop(winner);
            add_player_to_captain(
                draft_state.current_winner.as_ref().unwrap().clone(),
                draft_state.nominated_player.as_ref().unwrap().clone(),
            ).await;
            message2.edit(ctx,EditMessage::default().content(format!("{} bought {} for ${}",
                        draft_state.current_winner.as_ref().unwrap().lock().await.name,
                        draft_state.nominated_player.as_ref().unwrap().lock().await.name,
                        draft_state.current_bid,
            ))).await?;




        }
    }
    let captains = ctx.data().captains.lock().await;
    embed = make_final_draft_embed(captains.to_vec()).await;
    message.edit(ctx,EditMessage::default().embed(embed)).await?;

    Ok(())
}


#[tokio::main]
async fn main() {
    let user_data = UserData {
        captains: Mutex::new(Vec::new()),
        players: Mutex::new(import_players()),
        config: Mutex::new(Config::default()),
        draft_state: Mutex::new(DraftState::new()),
    };
    let token = std::env::var("DISCORD_TOKEN").expect("Missing Token");
    let intents = serenity::GatewayIntents::non_privileged();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                display_players(), 
                start_draft(), 
                add_captain(), 
                pick(),
                bid(),
                display_captains(),
                config(),
                display_teams(),
            ],
            ..Default::default()
        })
    .setup(|ctx, _ready, framework| {
        Box::pin(async move {
            poise::builtins::register_in_guild(ctx, 
                &framework.options().commands, 
                1318516792821420062.try_into().unwrap()).await?;
            Ok(user_data)
        })
    })
    .build();
    let client = serenity::ClientBuilder::new(token,intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}







