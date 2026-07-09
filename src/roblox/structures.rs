use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Creator {
    pub id: u64,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Universe {
    pub id: u64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub creator: Creator,
    pub playing: u32,
    pub visits: u64,
}