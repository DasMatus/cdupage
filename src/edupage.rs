use serde::{Serialize, Deserialize};
use reqwest::blocking::Client;

use crate::edupage_types::UserData;

#[derive(Clone)]
pub struct Edupage {
    is_logged_in: bool,
    client: reqwest::blocking::Client,
    data: Option<UserData>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
    pub csrfauth: String
}

impl LoginCredentials {
    pub fn new(username: String, password: String, csrfauth: String) -> Self { Self { username, password, csrfauth } }
}

#[derive(Debug)]
pub enum EdupageError {
    InvalidCredentials,
    HTTPError(String),
    InvalidResponse,
    ParseError(String),
    SerializationError(String)
}

impl Edupage {
    pub fn new() -> Self {
        let client = Client::builder()
            .connection_verbose(true)
            .cookie_store(true).build().unwrap();

        Self { is_logged_in: false, data: None, client }
    }

    fn parse_login_data(&mut self, html: String) -> Result<(), String> {
        let json = html.split("$j(document).ready(function() {").nth(1).unwrap()
            .split(");").nth(0).unwrap()
            .replace("\t", "")
            .split("userhome(").nth(1).unwrap()
            .replace("\n", "")
            .replace("\r", "");

        self.data = Some(match serde_json::from_str(&json) {
            Ok(x) => x,
            Err(e) => {
                return Err(e.to_string());
            }
        });

        Ok(())
    }

    pub fn logged_in(&self) -> bool {
        self.is_logged_in
    }

    pub fn login(&mut self, subdomain: &String, username: &String, password: &String) -> Result<(), EdupageError> {
        let url = format!("https://{}.edupage.org/login/index.php", subdomain);

        let request = self.client.get(url);
        let result = request.send();

        if result.is_err() {
            return Err(EdupageError::HTTPError(result.unwrap_err().to_string()));
        }

        let result = result.unwrap();
        let response_text = match result.text() {
            Ok(x) => x,
            Err(e) => return Err(EdupageError::HTTPError(e.to_string()))
        };

        if !response_text.contains("csrfauth") {
            return Err(EdupageError::InvalidResponse);
        }

        let csrf_token = match response_text.split("name=\"csrfauth\" value=\"").nth(1) {
            Some(x) => x,
            None => return Err(EdupageError::ParseError("Failed to parse csrf token.".to_string()))
        }.split("\"").nth(0).unwrap();

        let login_credentials = LoginCredentials::new(username.to_string(), password.to_string(), csrf_token.to_string());
        let post_data = match serde_urlencoded::to_string(&login_credentials) {
            Ok(x) => x,
            Err(e) => return Err(EdupageError::SerializationError(e.to_string()))
        };

        let url = format!("https://{}.edupage.org/login/edubarLogin.php", subdomain);
        let request = self.client.post(url)
            .body(post_data)
            // it took me 3 hours to figure out that this header is REQUIRED!!
            .header("Content-Type", "application/x-www-form-urlencoded");

        let result = request.send();

        if result.is_err() {
            return Err(EdupageError::HTTPError(result.unwrap_err().to_string()));
        }

        let result = result.unwrap();

        if result.url().as_str().contains("bad=1") {
            return Err(EdupageError::InvalidCredentials)
        }

        let response_text = match result.text() {
            Ok(x) => x,
            Err(e) => return Err(EdupageError::HTTPError(e.to_string()))
        };

        match self.parse_login_data(response_text.to_string()) {
            Ok(_) => {
                self.is_logged_in = true;
                Ok(())
            },
            Err(e) => {
                Err(EdupageError::ParseError(e.to_string()))
            }
        }


    }
}