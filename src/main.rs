mod roblox;

use roblox::structures::{Visibility, AgeRating, Universe};
use std::fmt::Write;
use tokio::{sync::mpsc, time::{Duration, Instant, sleep}};
use serde::Deserialize;
use base64::{engine::general_purpose, Engine as _};

const ROULLETE_UNIVERSE_ID: u64 = 10459051210;
const UNIVERSES_STORAGE_DATA_STORE_NAME: &str = "PlacesStorage";
const UNIVERSE_ID_ENDPOINT_PATH: &str = "https://apis.roblox.com/universes/v1/places/";
const UNIVERSE_DATA_ENDPOINT_PATH: &str = "https://apis.roblox.com/cloud/v2/universes/";
const DATA_STORE_ENTRY_COOLDOWN: Duration = Duration::from_secs(6);
const PLACES_SEARCHING_RANGE: core::ops::Range<u64> = 213497738..1_500_000_000;
const SEARCHING_TIME: Duration = Duration::from_secs(30 * 60 - 60);

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct UniverseIdResponse {
    universe_id: Option<u64>
}

#[tokio::main]
async fn main() {
    let server_start = Instant::now();
    let open_cloud_api_key = std::env::var("OPEN_CLOUD_API_KEY").unwrap();
    
    let client_1 = reqwest::Client::new();
    let client_2 = reqwest::Client::new();

    let mut universe_id_url = UNIVERSE_ID_ENDPOINT_PATH.to_string();
    let (id_sender, mut id_receiver) = mpsc::unbounded_channel::<u64>();

    tokio::spawn(async move {for place_id in PLACES_SEARCHING_RANGE {
        write!(&mut universe_id_url, "{}/universe", place_id).unwrap();
        let universe_id_response = client_2.get(&universe_id_url).send().await.unwrap();

        if let Some(universe_id) = universe_id_response.json::<UniverseIdResponse>().await.unwrap().universe_id {
            id_sender.send(universe_id).unwrap();
        }

        universe_id_url.truncate(UNIVERSE_ID_ENDPOINT_PATH.len());
    }});
    
    let mut universe_data_url = UNIVERSE_DATA_ENDPOINT_PATH.to_string();
    let mut universes_buffer: Vec<u8> = Vec::new();
    let mut bit_offset: usize = 18;
    let mut total_universes: u32 = 0;
    let mut universes_scanned: u64 = 0;

    while server_start.elapsed() < SEARCHING_TIME { 
        let universe_id = id_receiver.recv().await.unwrap();
        write!(&mut universe_data_url, "{}", universe_id).unwrap();

        let universe_data_response = client_1.get(&universe_data_url)
            .header("x-api-key", &open_cloud_api_key).send().await.unwrap();
            
        universe_data_url.truncate(UNIVERSE_DATA_ENDPOINT_PATH.len());

        if let Ok(universe) = universe_data_response.json::<Universe>().await {
            universes_scanned += 1;
            
            if !is_universe_public(&universe) {continue;}
            total_universes += 1;

            let creator_id = universe.user.strip_prefix("users/").unwrap().parse::<u64>().unwrap();
            let root_place_id = universe.root_place.rsplit('/').next().unwrap().parse::<u64>().unwrap();
            let name = &universe.display_name;
            let description = &universe.description;

            writebits(&mut universes_buffer, bit_offset, &universe_id.to_le_bytes(), 53);
            writebits(&mut universes_buffer, bit_offset+53, &root_place_id.to_le_bytes(), 53);
            writebits(&mut universes_buffer, bit_offset+106, &creator_id.to_le_bytes(), 53);
            writebits(&mut universes_buffer, bit_offset+159, &name.len().to_le_bytes(), 8);
            writebits(&mut universes_buffer, bit_offset+167, name.as_bytes(), name.len()*8);

            bit_offset += 167 + name.len() * 8;

            writebits(&mut universes_buffer, bit_offset, &description.len().to_le_bytes(), 12);
            writebits(&mut universes_buffer, bit_offset+12, description.as_bytes(), description.len()*8);
        
            bit_offset += 12 + description.len() * 8;
        }
    }

    writebits(&mut universes_buffer, 0, &total_universes.to_le_bytes(), 18);
    println!("Universes found: {}, scanned: {}", total_universes, universes_scanned);

    save_to_datastore(&client_1, open_cloud_api_key.as_str(), 
        UNIVERSES_STORAGE_DATA_STORE_NAME, "Page_1",
        &buffer_data_to_json(universes_buffer)
    ).await;
}

fn is_universe_public(universe: &Universe) -> bool {
    match universe.age_rating {AgeRating::AGE_RATING_13_PLUS => false, _ => {
        match universe.visibility {Visibility::PUBLIC => true, _ => false}
    }}
}

fn buffer_data_to_json(buf: Vec<u8>) -> String {
    let mut encoded = general_purpose::STANDARD.encode(buf);
    encoded.insert_str(0, "\"");
    encoded.push_str("\"");

    return encoded;
}

async fn save_to_datastore(client: &reqwest::Client, api_key: &str, 
    datastore_name: &str, key: &str, value_json: &String) 
{
    let digest = md5::compute(value_json.as_bytes());
    let content_md5 = general_purpose::STANDARD.encode(digest.as_ref());
    
    let url = format!(
        "https://apis.roblox.com/datastores/v1/universes/{}/standard-datastores/datastore/entries/entry?datastoreName={}&entryKey={}",
        ROULLETE_UNIVERSE_ID, datastore_name, key
    );

    loop {
        let Ok(resp) = client.post(&url)
            .header("x-api-key", api_key)
            .header("content-md5", &content_md5)
            .header("content-type", "application/json")
            .body(value_json.clone())
            .send()
            .await else {sleep(DATA_STORE_ENTRY_COOLDOWN).await; continue;};

        println!("status: {}, body: {}", resp.status(), resp.text().await.unwrap());
        break;
    }
}

fn writebits(buffer: &mut Vec<u8>, start_bit: usize, bytes: &[u8], bits_count: usize) {
    let end_byte = (start_bit + bits_count + 7) / 8;
    if buffer.len() < end_byte {
        buffer.resize(end_byte, 0);
    }

    for bit_i in 0..bits_count {
        let byte_i = bit_i / 8;
        let bit = (bytes[byte_i] >> (bit_i % 8)) & 1;

        let tarbit = start_bit + bit_i;
        let tarbyte = tarbit / 8;
        let tarshift = tarbit % 8;

        buffer[tarbyte] |= bit << tarshift;
    }
}