use crate::r0::AccessToken;
use reqwest::blocking::Client;
use reqwest::blocking::Request;
use reqwest::Error;
use ruma_identifiers::{RoomId, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub access_token: AccessToken,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Response {
    #[serde(default)]
    pub joined: HashMap<UserId, RoomMember>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RoomMember {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

pub fn request(base: Url, room_id: &RoomId, params: &Parameters) -> Result<Request, Error> {
    let url = base
        .join(&format!(
            "_matrix/client/r0/rooms/{}/joined_members",
            room_id
        ))
        .expect("Malformed URL in get_joined_members");

    Client::new().get(url).query(params).build()
}
