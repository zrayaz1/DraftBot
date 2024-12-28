use std::fmt::Display;

#[derive(Clone, Copy)]
pub struct Config {
    pub min_bid: u32,
    pub starting_balance: u32,
    pub team_size: u32,
    pub round_time: u32,
    pub bid_add_time: u32,
    pub legio_limit: u32,
}

impl Config {
    pub fn create() -> Self {
        Self {
            min_bid: 10,
            starting_balance: 200,
            team_size: 8,
            round_time: 20,
            bid_add_time: 5,
            legio_limit: 2,
        }
    }
}

impl From<&Config> for String {
    fn from(val: &Config) -> Self {
        String::from(*val)
    }
}

impl From<Config> for String {
    fn from(val: Config) -> Self {
        format!(
            "Min Bid: {}\nStarting Balance: {}\nTeam Size: {}\nRound Time: {}\nBid add team: {}\nLegio Limit: {}\n", 
            val.min_bid, 
            val.starting_balance, 
            val.team_size, 
            val.round_time, 
            val.bid_add_time, 
            val.legio_limit
        ).to_string()
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}
