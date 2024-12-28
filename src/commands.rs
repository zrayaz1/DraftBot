use poise::{serenity_prelude as serenity, CreateReply};
use rand::thread_rng;

use crate::{config, draft_state, make_final_draft_embed, Captain, Context, Error};

use crate::autocomplete_player;


#[poise::command(slash_command, prefix_command)]
pub async fn add_captain(
    ctx: Context<'_>,
    #[description = "Select User"] user: serenity::User,
    #[description = "Captain Name"] name: String,
) -> Result<(), Error> {
    let captain = Captain::new(name.clone(), config.starting_balance, u64::from(user.id));

    let _ = ctx
        .send(
            CreateReply::default()
                .content(
                    match draft_state.teams.try_insert(captain, vec![]) {
                        Ok(_) => format!("Added captain {}", name),
                        Err(_) => format!("Already added captain {}", name)
                    }
                )
                .reply(true)
                .ephemeral(false),
            )
            .await;

    Ok(())
}

/// Displays Captains, Use Display Teams instead
#[poise::command(slash_command)]
pub async fn display_captains(ctx: Context<'_>) -> Result<(), Error> {
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
pub async fn set_config(
    ctx: Context<'_>,
    #[description = "Round Time"] round_time: Option<u32>,
    #[description = "Bid Add Time"] bid_add_time: Option<u32>,
    #[description = "Minimum Bid"] min_bid: Option<u32>,
    #[description = "Starting Balance"] starting_balance: Option<u32>,
    #[description = "Team Size"] team_size: Option<u32>,
    #[description = "Legio Limit"] legio_limit: Option<u32>,
) -> Result<(), Error> {
    config.round_time = round_time.unwrap_or(config.round_time);
    config.bid_add_time = bid_add_time.unwrap_or(config.bid_add_time);
    config.min_bid = min_bid.unwrap_or(config.min_bid);

    config.legio_limit = legio_limit.unwrap_or(config.legio_limit);

    if starting_balance.is_some() {
        if draft_state.draft_started {
            let _ = ctx
                .send(
                    CreateReply::default()
                        .content("Cannot change starting balance after start")
                        .reply(true)
                        .ephemeral(true),
                )
                .await;
            return Ok(());
        }
        config.starting_balance = starting_balance.unwrap_or(config.starting_balance);
    }

    if team_size.is_some() {
        if draft_state.draft_started {
            let _ = ctx
                .send(
                    CreateReply::default()
                        .content("Cannot change team size after start")
                        .reply(true)
                        .ephemeral(true),
                )
                .await;
            return Ok(());
        }
        config.team_size = team_size.unwrap_or(config.team_size);
    }

    if legio_limit.is_some() && draft_state.draft_started {
        let _ =
            ctx.say("This might not function properly after draft start. Restart draft if early.");
    }

    let _ = ctx.say(config).await;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn display_teams(ctx: Context<'_>) -> Result<(), Error> {
    let embed = make_final_draft_embed().await;
    let _ = ctx.send(CreateReply::default().embed(embed)).await;
    Ok(())
}

/// Displays players (for debugging do not use) DO NOT USE
#[poise::command(slash_command)]
pub async fn display_players(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say(
        draft_state
            .players
            .iter()
            .map(|player| format!("{} ", player).to_string())
            .collect::<String>(),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn pick(
    ctx: Context<'_>,
    #[description = "Select a Player"]
    #[autocomplete = "autocomplete_player"]
    player: String,
    #[description = "Starting Bid"] starting_bid: Option<u32>,
) -> Result<(), Error> {
    if draft_state.time != config.round_time {
        let _ = ctx
            .send(
                CreateReply::default()
                    .content("Only pick before round starts")
                    .reply(true)
                    .ephemeral(true),
            )
            .await;
        return Ok(());
    }
    let mut final_bid = config.min_bid;
    drop(config);
    match current_captain {
        Some(captain) => {
            let c1 = captain.clone();
            let c = c1.lock().await;
            if ctx.author().id != c.discord_id {
                let _ = ctx
                    .send(
                        CreateReply::default()
                            .content("You not da captain blud")
                            .reply(true)
                            .ephemeral(true),
                    )
                    .await;
                return Ok(());
            }
            let config = ctx.data().config.lock().await;
            max_bid = c.get_max_bid(&config);
            draft_state.current_winner = Some(captain.clone());
        }
        None => {
            let _ = ctx
                .send(
                    CreateReply::default()
                        .content("No Draft Running")
                        .reply(true)
                        .ephemeral(true),
                )
                .await;
            return Ok(());
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
                    let _ = ctx
                        .send(
                            CreateReply::default()
                                .content(format!("Not enough funds, Your max bid is:{}", max_bid))
                                .reply(true)
                                .ephemeral(true),
                        )
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


#[poise::command(slash_command)]
pub async fn bid(ctx: Context<'_>, #[description = "Amount"] amount: u32) -> Result<(), Error> {
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
                let _ = ctx
                    .send(
                        CreateReply::default()
                            .content("Not in captain list")
                            .reply(true)
                            .ephemeral(true),
                    )
                    .await;
                return Ok(());
            }
            if amount > max_bid {
                ctx.send(
                    CreateReply::default()
                        .content(format!("Not enough funds, Your max bid is:{}", max_bid))
                        .reply(true)
                        .ephemeral(true),
                )
                .await?;

                return Ok(());
            }
            draft_state.current_bid = amount;
            draft_state.current_winner = Some(captain.unwrap());
            draft_state.bid_placed = true;
            let msg = ctx
                .send(
                    CreateReply::default()
                        .content(format!(
                            "You Bid ${} for {}",
                            amount,
                            draft_state
                                .nominated_player
                                .as_ref()
                                .unwrap()
                                .lock()
                                .await
                                .name
                        ))
                        .reply(true)
                        .ephemeral(true),
                )
                .await?;
            drop(draft_state);
            msg.delete(ctx).await?;
            return Ok(());
        } else {
            ctx.send(
                CreateReply::default()
                    .content("Under current top bid")
                    .reply(true)
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    } else {
        let _ = ctx
            .send(
                CreateReply::default()
                    .content("No Auction Running")
                    .reply(true)
                    .ephemeral(true),
            )
            .await;
        return Ok(());
    }
}

#[poise::command(slash_command)]
pub async fn start_draft(ctx: Context<'_>) -> Result<(), Error> {
    let _ = ctx.defer().await;
    captains_main.shuffle(&mut thread_rng());
    let team_size = config.team_size;
    let captains = captains_main.clone();
    drop(captains_main);
    let mut draft_state = ctx.data().draft_state.lock().await;
    let mut embed = generate_draft_embed(&draft_state).await;
    draft_state.draft_started = true;
    drop(draft_state);
    drop(config);
    let mut message = ctx
        .send(CreateReply::default().embed(embed))
        .await?
        .into_message()
        .await?;
    let mut message2 = ctx
        .send(CreateReply::default().content("Draft Started"))
        .await?
        .into_message()
        .await?;
    for i in 1..team_size + 1 {
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
            message
                .edit(ctx, EditMessage::default().embed(embed))
                .await?;
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
                    message
                        .edit(ctx, EditMessage::default().embed(embed))
                        .await?;
                }
                if draft_state.nominated_player.is_some() {
                    break;
                }
            }
            let config = ctx.data().config.lock().await;
            let draft_state = ctx.data().draft_state.lock().await;
            embed = generate_draft_embed(&draft_state).await;
            message2
                .edit(
                    ctx,
                    EditMessage::default().content(format!(
                        "{} bid {} for {}",
                        draft_state
                            .current_winner
                            .as_ref()
                            .unwrap()
                            .lock()
                            .await
                            .name,
                        draft_state.current_bid,
                        draft_state
                            .nominated_player
                            .as_ref()
                            .unwrap()
                            .lock()
                            .await
                            .name,
                    )),
                )
                .await?;
            drop(draft_state);
            message
                .edit(ctx, EditMessage::default().embed(embed))
                .await?;
            let mut time_left: u32 = config.round_time;
            drop(config);
            while time_left > 0 {
                interval.tick().await;
                let mut draft_state = ctx.data().draft_state.lock().await;
                if time_left % 2 != 0 {
                    time_left += 1;
                }
                time_left = time_left - 2;
                draft_state.time = time_left;
                embed = generate_draft_embed(&draft_state).await;
                message
                    .edit(ctx, EditMessage::default().embed(embed))
                    .await?;
                if draft_state.bid_placed == true {
                    let config = ctx.data().config.lock().await;
                    time_left = time_left + config.bid_add_time;
                    draft_state.bid_placed = false;
                    message2
                        .edit(
                            ctx,
                            EditMessage::default().content(format!(
                                "{} bid {} for {}",
                                draft_state
                                    .current_winner
                                    .as_ref()
                                    .unwrap()
                                    .lock()
                                    .await
                                    .name,
                                draft_state.current_bid,
                                draft_state
                                    .nominated_player
                                    .as_ref()
                                    .unwrap()
                                    .lock()
                                    .await
                                    .name,
                            )),
                        )
                        .await?;
                }
            }
            let draft_state = ctx.data().draft_state.lock().await;
            let mut winner = draft_state.current_winner.as_ref().unwrap().lock().await;
            winner.balance = winner.balance - draft_state.current_bid;
            drop(winner);
            add_player_to_captain(
                draft_state.current_winner.as_ref().unwrap().clone(),
                draft_state.nominated_player.as_ref().unwrap().clone(),
            )
            .await;
            message2
                .edit(
                    ctx,
                    EditMessage::default().content(format!(
                        "{} bought {} for ${}",
                        draft_state
                            .current_winner
                            .as_ref()
                            .unwrap()
                            .lock()
                            .await
                            .name,
                        draft_state
                            .nominated_player
                            .as_ref()
                            .unwrap()
                            .lock()
                            .await
                            .name,
                        draft_state.current_bid,
                    )),
                )
                .await?;
        }
    }
    embed = make_final_draft_embed().await;
    message
        .edit(ctx, EditMessage::default().embed(embed))
        .await?;

    Ok(())
}