use dotenv::dotenv;
use egg_mode;
use egg_mode::tweet::DraftTweet;
use egg_mode::Token;
use reqwest;
use reqwest::Error;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use tokio::time::{self, Duration};

// Format of OpenSea's API response (https://api.opensea.io/api/v1/collection/pawnbots/stats)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ApiResponse {
    stats: HashMap<String, f64>,
}

// Contains all the necessary data to tweet, extracted from OpenSea's API
#[derive(Clone, Debug)]
pub struct CollectionStats {
    pub one_day_volume: f64,
    pub seven_day_average_price: f64,
    pub market_cap: f64,
    pub floor_price: f64,
    pub num_owners: f64,
}

impl CollectionStats {
    // Simple constructor to create a new instance of the struct
    pub fn new(api_response: ApiResponse) -> CollectionStats {
        CollectionStats {
            one_day_volume: round_value(api_response.stats["one_day_volume"]),
            seven_day_average_price: round_value(api_response.stats["seven_day_average_price"]),
            market_cap: round_value(api_response.stats["market_cap"]),
            floor_price: round_value(api_response.stats["floor_price"]),
            num_owners: round_value(api_response.stats["num_owners"]),
        }
    }
}

// Returns a rounded value with 3 decimals (example: 2.236067 --> 2.236)
fn round_value(unrounded_value: f64) -> f64 {
    (1000.0 * unrounded_value).round() / 1000.0
}

// Gets the required data to tweet from OpenSea (no API key required)
async fn opensea_api_request() -> Result<CollectionStats, Error> {
    let request_url = "https://api.opensea.io/api/v1/collection/pawnbots/stats";
    let response = reqwest::get(request_url).await?.text().await?;

    let api_response: ApiResponse = serde_json::from_str(response.as_str())
        .expect("OpenSea's API response was not well-formatted");

    let pawn_bot_metrics = CollectionStats::new(api_response);

    Ok(pawn_bot_metrics)
}

// Returns the message to tweet properly formatted
fn format_message(pawn_bot_metrics: CollectionStats) -> String {
    format!("🤖 The current floor price is {} ETH, with a daily volume of {} ETH and a market cap of {} ETH.\n\nThe weekly average sell price is {} ETH, and there are currently {} holders.", pawn_bot_metrics.floor_price, pawn_bot_metrics.one_day_volume, pawn_bot_metrics.market_cap, pawn_bot_metrics.seven_day_average_price, pawn_bot_metrics.num_owners).to_string()
}

// Tweets the message and prints an error if Twitter returns one
async fn tweet(token: &Token, message: String) {
    match DraftTweet::new(message).send(&token).await {
        Ok(..) => (),
        Err(e) => println!("Error: {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();

    // API keys from the app
    let consumer_token = egg_mode::KeyPair::new(
        env::var("TWITTER_CONSUMER_KEY").expect("Expected TWITTER_CONSUMER_KEY in environment"),
        env::var("TWITTER_CONSUMER_SECRET")
            .expect("Expected TWITTER_CONSUMER_SECRET in environment"),
    );

    // Oauth keys from the account being tweeted from
    let access_token = egg_mode::KeyPair::new(
        env::var("TWITTER_ACCESS_TOKEN").expect("Expected TWITTER_ACCESS_TOKEN in environment"),
        env::var("TWITTER_ACCESS_TOKEN_SECRET")
            .expect("Expected TWITTER_ACCESS_TOKEN_SECRET in environment"),
    );

    // Create a token to authenticate  and tweet using the egg_mode crate
    let token = egg_mode::Token::Access {
        consumer: consumer_token,
        access: access_token,
    };

    // Create an interval of 1 day so the bot tweets every day
    let mut interval = time::interval(Duration::from_secs(86_400));

    loop {
        // Wait 1 day
        interval.tick().await;
        match opensea_api_request().await {
            // If the opensea_api_request() function returns a valid response, send a tweet out
            Ok(pawn_bot_metrics) => tweet(&token, format_message(pawn_bot_metrics)).await,
            // If not, print the error message
            Err(e) => println!("Error: {}", e),
        }
    }
}
