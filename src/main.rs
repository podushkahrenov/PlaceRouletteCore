mod roblox;

use std::fmt::Write;
use reqwest::Response;
use tokio::time::{sleep, Duration, Instant};
use serde::{Deserialize};

const SEARCHING_RANGE: core::ops::Range<u64> = 2000..10000000;
const SEARCHING_INTERVAL: Duration = Duration::from_millis(700);
const SEARCHING_TIMEOUT: Duration = Duration::from_secs(10);
const SEARCHING_TIME: Duration = Duration::from_mins(1);

#[derive(Deserialize, Debug)]
struct UniversesDataResponse {
    data: Vec<roblox::structures::Universe>
}

#[derive(Deserialize, Debug)]
struct UniverseIdResponse {
    universeId: Option<u64>
}

#[tokio::main]
async fn main() {
    let server_start = Instant::now();

    let client = reqwest::Client::new();
    let mut universes: Vec<u64> = Vec::with_capacity(1000);

    let mut url_buf = "https://apis.roblox.com/universes/v1/places/".to_string();
    let url_buf_len = url_buf.len();
    
    url_buf.reserve((1 + u64::MAX.ilog10() as usize) + "/universe".len());

    for place_id in SEARCHING_RANGE {
        write!(&mut url_buf, "{}/universe", place_id).unwrap();
        
        if let Some(response) = get_roblox_response(&client, url_buf.as_str()).await {
            if let Ok(parsed) = response.json::<UniverseIdResponse>().await {
                if let Some(universe_id) = parsed.universeId {universes.push(universe_id)}
            }
        }

        url_buf.truncate(url_buf_len);

        if server_start.elapsed() >= SEARCHING_TIME {break;}
    }

    let total_universes = universes.len();
    println!("Universes found: {}", total_universes);


    let mut url_buf = "https://games.roblox.com/v1/games?universeIds=".to_string();
    let url_buf_len = url_buf.len();

    url_buf.reserve(50 * (u64::MAX.ilog10() + 1 + 1) as usize);

    let mut bytes: Vec<u8> = Vec::with_capacity(total_universes / 2);
    let mut last_bit: u32 = 0;

    for i in (0..total_universes).step_by(50) {
        let universes_slice = &universes[i..(i+50).min(total_universes)];

        for universe_id in universes_slice {
            write!(url_buf, "{},", universe_id).unwrap();
        }

        url_buf.pop();

        if let Some(response) = get_roblox_response(&client, url_buf.as_str()).await {
            if let Ok(parsed) = response.json::<UniversesDataResponse>().await {
                println!("{:?}", parsed.data);
            }
        }

        url_buf.truncate(url_buf_len);
    }

}

async fn get_roblox_response(client: &reqwest::Client, url: &str) -> Option<Response> {
    return loop { sleep(SEARCHING_INTERVAL).await;

        if let Ok(response) = client.get(url).send().await { match response.status() {
            reqwest::StatusCode::OK => break Some(response),

            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                sleep(SEARCHING_TIMEOUT).await;
            }

            _ => break None
        }}
    }
}