mod roblox;
use tokio::time::{sleep, Duration, Instant};
use serde::Deserialize;

const SEARCHING_RANGE: core::ops::Range<u64> = 2000..10000000;
const SEARCHING_INTERVAL: Duration = Duration::from_millis(700);
const SEARCHING_TIMEOUT: Duration = Duration::from_secs(10);
const SEARCHING_TIME: Duration = Duration::from_mins(60*6-20);

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

    for place_id in SEARCHING_RANGE {
        let url = format!("https://apis.roblox.com/universes/v1/places/{}/universe", place_id);
        
        loop { sleep(SEARCHING_INTERVAL).await;

            if let Ok(resp) = client.get(&url).send().await { match resp.status() {
                reqwest::StatusCode::OK => {
                    let parsed = resp.json::<UniverseIdResponse>().await.unwrap();
                    if let Some(universe_id) = parsed.universeId {universes.push(universe_id);}

                    break;
                }
                reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    sleep(SEARCHING_TIMEOUT).await
                }
                _ => {}
            }}
        }

        if server_start.elapsed() >= SEARCHING_TIME {break;}
    }

    println!("Universes found: {}", universes.len());

}