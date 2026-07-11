use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Creator {
    pub id: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Universe {
    pub id: u64,
    pub root_place_id: u64,
    pub creator: Creator,
    pub name: String,
    pub description: Option<String>,
}