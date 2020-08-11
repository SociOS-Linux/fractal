use crate::r0::AccessToken;
use reqwest::blocking::Client;
use reqwest::blocking::Request;
use reqwest::Error;
use ruma_identifiers::RoomId;
use serde::Serialize;
use serde_json::Value as JsonValue;
use url::Url;

#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub access_token: AccessToken,
}

pub fn request(
    base: Url,
    params: &Parameters,
    room_id: &RoomId,
    rule: &JsonValue,
) -> Result<Request, Error> {
    let url = base
        .join(&format!(
            "_matrix/client/r0/pushrules/global/room/{}",
            room_id
        ))
        .expect("Malformed URL in set_room_rules");

    Client::new().put(url).query(params).json(rule).build()
}
