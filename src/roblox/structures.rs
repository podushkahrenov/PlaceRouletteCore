use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub enum Visibility {
    VISIBILITY_UNSPECIFIED,
    PUBLIC,
    PRIVATE
}

#[derive(Deserialize, Debug)]
pub enum AgeRating {
    AGE_RATING_UNSPECIFIED,
    AGE_RAING_ALL,
    AGE_RATING_9_PLUS,
    AGE_RATING_13_PLUS,
    AGE_RATING_17_PLUS
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Universe {
    pub display_name: String,
    pub root_place: String,
    pub user: String,
    pub description: String,
    pub age_rating: AgeRating,
    pub visibility: Visibility
}