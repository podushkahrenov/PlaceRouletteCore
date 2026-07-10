mod roblox;

use std::{fmt::Write, vec};
use reqwest::Response;
use tokio::time::{sleep, Duration, Instant};
use serde::{Deserialize};
use base64::{engine::general_purpose, Engine as _};

const ROULLETE_UNIVERSE_ID: u64 = 10459051210;
const UNIVERSES_STORAGE_DATA_STORE: &str = "PlacesStorage";
const REQUEST_INTERVAL: Duration = Duration::from_millis(700);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const SEARCHING_RANGE: core::ops::Range<u64> = 2000..10000000;
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
    let open_cloud_api_key = std::env::var("OPEN_CLOUD_API_KEY").unwrap();
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

    let mut universes_data_bytes: Vec<u8> = vec![0; total_universes * 40];
    let mut last_bit: usize = 0;

    for i in (0..total_universes).step_by(50) {
        let universes_slice = &universes[i..(i+50).min(total_universes)];

        for universe_id in universes_slice {
            write!(url_buf, "{},", universe_id).unwrap();
        }

        if let Some(response) = get_roblox_response(&client, url_buf.as_str()).await {
            if let Ok(parsed) = response.json::<UniversesDataResponse>().await {
                
                for universe in &parsed.data {
                    let name = &universe.name;
                    let name_len = name.len();

                    insert_bits(&mut universes_data_bytes, last_bit, &name_len.to_le_bytes(), 8);
                    insert_bits(&mut universes_data_bytes, last_bit + 8, name.as_bytes(), name_len * 8);
                    last_bit += 8 + name_len * 8
                }
            }
        }

        url_buf.truncate(url_buf_len);
    }

    let mut universes_data_base64 = general_purpose::STANDARD.encode(universes_data_bytes);
    universes_data_base64.reserve("\"".len() * 2);
    universes_data_base64.insert_str(0, "\"");
    universes_data_base64.push_str("\"");

    save_to_datastore(&client, open_cloud_api_key.as_str(), 
        UNIVERSES_STORAGE_DATA_STORE, "Page_1", 
        universes_data_base64
    ).await;

}

async fn get_roblox_response(client: &reqwest::Client, url: &str) -> Option<Response> {
    return loop { sleep(REQUEST_INTERVAL).await;

        if let Ok(response) = client.get(url).send().await { match response.status() {
            reqwest::StatusCode::OK => break Some(response),

            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                sleep(REQUEST_TIMEOUT).await;
            }

            _ => break None
        }}
    }
}

async fn save_to_datastore(client: &reqwest::Client, api_key: &str, 
    datastore_name: &str, key: &str, value_json: String) 
{
    let digest = md5::compute(value_json.as_bytes());
    let content_md5 = general_purpose::STANDARD.encode(digest.as_ref());
    
    let url = format!(
        "https://apis.roblox.com/datastores/v1/universes/{}/standard-datastores/datastore/entries/entry?datastoreName={}&entryKey={}",
        ROULLETE_UNIVERSE_ID, datastore_name, key
    );

    let resp = client.post(&url)
        .header("x-api-key", api_key)
        .header("content-md5", content_md5)
        .header("content-type", "application/json")
        .body(value_json)
        .send()
        .await.unwrap();

    println!("status: {}, body: {}", resp.status(), resp.text().await.unwrap());
}

fn insert_bits(buffer: &mut Vec<u8>, bit_pos: usize, bytes: &[u8], bits_count: usize) {
    let byte_pos = bit_pos / 8;
    let shift = bit_pos as u32 % 8;

    for byte_i in 0..bits_count / 8 {
        let byte = bytes[byte_i];
        let pos = byte_pos + byte_i;

        if shift > 0 {
            buffer[pos] |= byte.wrapping_shr(shift);
            buffer[pos + 1] |= byte.wrapping_shl(8 - shift);
        } else {
            buffer[pos] |= byte;
        }
    }

    let tail_bits = bits_count % 8; if tail_bits > 0 {
        let byte_i = bits_count / 8;
        let filter: u8 = (1 << tail_bits) - 1;

        let aligned = (bytes[byte_i] & filter) << (8 - tail_bits as u32);
        let pos = byte_pos + byte_i;

        if shift > 0 {
            buffer[pos] |= aligned.wrapping_shr(shift);
            buffer[pos + 1] |= aligned.wrapping_shl(8 - shift);
        } else {
            buffer[pos] |= aligned;
        }
    }
}