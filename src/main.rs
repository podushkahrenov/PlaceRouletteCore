mod roblox;

use std::fmt::Write;
use roblox::structures::{Visibility, AgeRating, Universe};
use tokio::time::{Duration, Instant, sleep};
use base64::{engine::general_purpose, Engine as _};

const ROULLETE_UNIVERSE_ID: u64 = 10459051210;
const UNIVERSES_STORAGE_DATA_STORE_NAME: &str = "PlacesStorage";
const UNIVERSES_SEARCHING_RANGE: core::ops::Range<u64> = 10459051200..1_500_000_0000;
const SEARCHING_TIME: Duration = Duration::from_secs(3 * 60 - 60);
const UNIVERSE_DATA_ENDPOINT_PATH: &str = "https://apis.roblox.com/cloud/v2/universes/";

#[derive(serde::Deserialize, Debug)]
struct DataStoreEntryResponse {
    value: String
}

#[tokio::main]
async fn main() {
    let server_start = Instant::now();

    let open_cloud_api_key = std::env::var("OPEN_CLOUD_API_KEY").unwrap();
    let client = reqwest::Client::new();

    let mut universe_data_url = UNIVERSE_DATA_ENDPOINT_PATH.to_string();
    let mut total_universes: u64 = 0;
    let mut universes_scanned: u64 = 0;

    let mut universes_buffer = general_purpose::STANDARD.decode(
        get_datastore_entry(&client, open_cloud_api_key.as_str(), 
        UNIVERSES_STORAGE_DATA_STORE_NAME, "Page_1"
    ).await.value).unwrap();

    let mut bit_offset: usize = 18;

    for universe_id in UNIVERSES_SEARCHING_RANGE {
        if server_start.elapsed() >= SEARCHING_TIME {break;}
        universes_scanned += 1;

        write!(&mut universe_data_url, "{}", universe_id).unwrap();
        let universe_data_response = client.get(&universe_data_url)
            .header("x-api-key", &open_cloud_api_key).send().await.unwrap();
        
        universe_data_url.truncate(UNIVERSE_DATA_ENDPOINT_PATH.len());

        if let Ok(universe) = universe_data_response.json::<Universe>().await {
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

    incrementbits(&mut universes_buffer, 0, total_universes, 18);
    println!("Universes found: {}, scanned: {}", total_universes, universes_scanned);

    save_to_datastore(&client, open_cloud_api_key.as_str(), 
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
        "https://apis.roblox.com/cloud/v2/universes/{}/data-stores/{}/entries/{}",
        ROULLETE_UNIVERSE_ID, datastore_name, key
    );
    
    loop {
        let Ok(resp) = client.patch(&url)
            .header("x-api-key", api_key)
            .header("content-md5", &content_md5)
            .header("content-type", "application/json")
            .body(value_json.clone())
            .send()
            .await else {continue;};

        println!("status: {}, body: {}", resp.status(), resp.text().await.unwrap());
        break;
    }
}

async fn get_datastore_entry(client: &reqwest::Client, api_key: &str, 
    datastore_name: &str, key: &str) -> DataStoreEntryResponse
{
    let url = format!(
        "https://apis.roblox.com/cloud/v2/universes/{}/data-stores/{}/entries/{}",
        ROULLETE_UNIVERSE_ID, datastore_name, key
    );

    let response = client.get(&url)
        .header("x-api-key", api_key).send().await.unwrap();

    response.json::<DataStoreEntryResponse>().await.unwrap()
}

fn incrementbits(buffer: &mut Vec<u8>, start_bit: usize, amount: u64, bits_count: usize) {
    let value = readbits(buffer, start_bit, bits_count);
    writebits(buffer, start_bit, &(value + amount).to_le_bytes(), bits_count);
}

fn readbits(buffer: &Vec<u8>, start_bit: usize, bits_count: usize) -> u64 {
    let mut result: u64 = 0;

    for bit_i in 0..bits_count {
        let bit_pos = start_bit + bit_i;
        let bit = (buffer[bit_pos / 8] >> (7 - bit_pos % 8)) & 1;

        result = (result << 1) | bit as u64;
    }

    return result;
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