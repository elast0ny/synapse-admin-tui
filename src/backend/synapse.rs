use std::{borrow::Cow, ops::Deref};

use reqwest::{Method, StatusCode, blocking::{Client, Response}};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    editable::EditableWidget,
    view::{PromptButton, PromptInfo},
};

pub struct Synapse {
    token_valid: bool,
    access_token: String,
    host: String,
    client: Client,
    url_cache: String,
    body_cache: String,
}

impl Synapse {
    pub fn new(host: String, allow_invalid_certs: bool) -> Self {
        Self {
            token_valid: false,
            host,
            access_token: String::new(),
            client: Client::builder()
                .user_agent(format!(
                    "{} ({})",
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION")
                )).danger_accept_invalid_certs(allow_invalid_certs)
                .build()
                .unwrap(),
            url_cache: String::with_capacity(128),
            body_cache: String::with_capacity(128),
        }
    }

    pub fn update_prompt(&mut self, prompt: &mut PromptInfo) -> bool {
        if self.token_valid {
            return false;
        }

        if let Some(a) = prompt.action.take() {
            match a {
                PromptButton::Exit => return false,
                _ => {}
            }

            // Grab the values from the input fields
            self.host.clear();
            self.host.push_str(prompt.fields[0].1.as_str().trim_end_matches('/'));
            self.access_token.clear();
            self.access_token.push_str(prompt.fields[1].1.as_str());

            if let Err(e) = self.validate_token() {
                // Try to set the cursor to the invalid input
                if e.contains("401") {
                    prompt.cursor = 1;
                } else if e.contains("request") {
                    prompt.cursor = 0;
                }
                prompt.bottom.clear();
                prompt.bottom.push_str(e.deref());
                return true;
            }

            self.token_valid = true;
            return false;
        }

        prompt.clear();
        if !self.host.is_empty() {
            prompt.cursor = 1;
        }

        prompt
            .top
            .push_str("Please provide the missing server information");
        prompt
            .fields
            .push(("Host".into(), self.host.clone().into()));
        prompt
            .fields
            .push(("Access Token".into(), self.access_token.clone().into()));

        prompt.buttons.push(PromptButton::Ok);
        prompt.buttons.push(PromptButton::Exit);

        return true;
    }

    pub fn validate_token(&mut self) -> Result<(), Cow<'static, str>> {
        if self.host.is_empty() {
            return Err("Error : host is mandatory".into());
        } else if self.access_token.is_empty() {
            return Err("Error : token is mandatory".into());
        }
        
        // Try to use a random api
        // There may be an endpoint with its sole purpose is to validate the access token but i couldnt find it
        let r = self.send::<&str, ResetPasswordV1>(Method::GET, "_synapse/admin/v1/username_available?username=a", None, Some(StatusCode::OK))?;

        let resp = match r.text() {
            Ok(v) => v,
            Err(_e) => return Err("Server response is invalid".into()),
        };

        Ok(())
    }

    pub fn list_users(&mut self, offset: usize, page_size: usize) -> Result<Vec<UserInfoV1>, String> {

        let url = format!("_synapse/admin/v2/users?from={}&limit={}&guests=false", offset, page_size);
        let r = self.send::<_, ResetPasswordV1>(Method::GET, url, None, Some(StatusCode::OK))?;
        
        let resp = match r.text() {
            Ok(v) => v,
            Err(_e) => return Err("Server response is invalid".into()),
        };

        let data: ListUserV1 = match serde_json::from_str(&resp) {
            Ok(v) => v,
            Err(e) => return Err(format!("Server response is invalid\n{}\n{}", resp, e).into()),
        };

        Ok(data.users)
    }

    fn send<'b, P: Into<Cow<'static, str>>, S: Serialize>(&'b mut self, method: Method, path: P, body: Option<S>, expected_status: Option<StatusCode>) -> Result<Response, Cow<'static, str>> {
        use std::fmt::Write;
        self.url_cache.clear();
        
        let _ = write!(&mut self.url_cache, "{}/{}", self.host.as_str(), path.into().deref());
        let mut req = self.client.request(method.clone(), self.url_cache.as_str()).bearer_auth(self.access_token.as_str());
        
        if let Some(b) = body {
            self.body_cache.clear();
            if let Err(_e) = serde_json::to_writer(unsafe{self.body_cache.as_mut_vec()}, &b) {
                return Err("Request body is invalid json".into())
            }
            req = req.body(self.body_cache.clone());
        }

        let resp = match req.send() {
            Ok(r) => r,
            Err(e) => return Err(format!("{}", e).into()),
        };

        if let StatusCode::UNAUTHORIZED = resp.status() {
            self.token_valid = false;
        }

        if let Some(s) = expected_status {
            if resp.status() != s {
                return Err(format!("{} {} returned {}", method, self.url_cache, resp.status()).into())
            }
        }

        Ok(resp)
    }
}

#[derive(Default, Serialize)]
struct AccountValidityV1 {
    user_id: Cow<'static, str>,
    #[serde(skip_serializing_if="Option::is_none")]
    expiration_ts: Option<usize>,
    #[serde(skip_serializing_if="Option::is_none")]
    enable_renewal_emails: Option<bool>,
}

#[derive(Default, Serialize)]
struct ResetPasswordV1 {
    new_password: String,
    #[serde(skip_serializing_if="Option::is_none")]
    logout_devices: Option<bool>,
}

#[derive(Default, Deserialize)]
struct ListUserV1 {
    pub users: Vec<UserInfoV1>,
    pub next_token: Option<usize>,
    pub total: usize,
}

#[derive(Default, Deserialize)]
pub struct UserInfoV1 {
    pub name: String,
    #[serde(deserialize_with="bool_from_num")]
    pub is_guest: bool,
    #[serde(deserialize_with="bool_from_num")]
    pub admin: bool,
    pub user_type: serde_json::Value,
    #[serde(deserialize_with="bool_from_num")]
    pub deactivated: bool,
    pub shadow_banned: bool,
    pub displayname: String,
    pub avatar_url: Option<String>,
    pub creation_ts: usize,
}

fn bool_from_num<'de, D>(deserializer: D) -> Result<bool, D::Error>
where D: Deserializer<'de> {
    let v = usize::deserialize(deserializer)?;

    if v == 0 {
        Ok(false)
    } else {
        Ok(true)
    }
}
