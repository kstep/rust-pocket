#![allow(unstable)]

extern crate hyper;
extern crate "rustc-serialize" as rustc_serialize;
extern crate url;
extern crate mime;

#[cfg(test)] #[macro_use] extern crate log;

use hyper::header::{Header, HeaderFormat, ContentType};
use hyper::client::Client;
use hyper::net::HttpConnector;
use hyper::HttpError;
use hyper::header::shared::util::from_one_raw_str;
use url::Url;
use mime::Mime;
use rustc_serialize::json;
use std::error::{FromError, Error};
use std::io::IoError;

#[derive(Show)]
pub enum PocketError {
    Http(HttpError),
    Json(json::DecoderError)
}

pub type PocketResult<T> = Result<T, PocketError>;

impl FromError<json::DecoderError> for PocketError {
    fn from_error(err: json::DecoderError) -> PocketError {
        PocketError::Json(err)
    }
}

impl FromError<IoError> for PocketError {
    fn from_error(err: IoError) -> PocketError {
        PocketError::Http(FromError::from_error(err))
    }
}

impl FromError<HttpError> for PocketError {
    fn from_error(err: HttpError) -> PocketError {
        PocketError::Http(err)
    }
}

impl Error for PocketError {
    fn description(&self) -> &str {
        match *self {
            PocketError::Http(ref e) => e.description(),
            PocketError::Json(ref e) => e.description(),
        }
    }

    fn detail(&self) -> Option<String> {
        match *self {
            PocketError::Http(ref e) => e.detail(),
            PocketError::Json(ref e) => e.detail(),
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            PocketError::Http(ref e) => Some(e),
            PocketError::Json(ref e) => Some(e)
        }
    }
}

#[derive(Clone)]
struct XAccept(pub Mime);

impl std::ops::Deref for XAccept {
    type Target = Mime;
    fn deref<'a>(&'a self) -> &'a Mime {
        &self.0
    }
}

impl std::ops::DerefMut for XAccept {
    fn deref_mut<'a>(&'a mut self) -> &'a mut Mime {
        &mut self.0
    }
}

impl Header for XAccept {
    #[allow(unused_variables)]
    fn header_name(marker: Option<Self>) -> &'static str {
        "X-Accept"
    }

    fn parse_header(raw: &[Vec<u8>]) -> Option<XAccept> {
        from_one_raw_str(raw).map(|mime| XAccept(mime))
    }
}

impl HeaderFormat for XAccept {
    fn fmt_header(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::String::fmt(&self.0, fmt)
    }
}

pub struct Pocket<'a> {
    consumer_key: String,
    access_token: Option<String>,
    code: Option<String>,
    client: Client<HttpConnector<'a>>
}

#[derive(RustcEncodable)]
struct PocketOAuthRequest<'a> {
    consumer_key: &'a str,
    redirect_uri: &'a str,
    state: Option<&'a str>
}

#[derive(RustcDecodable)]
struct PocketOAuthResponse {
    code: String,
    state: Option<String>
}

#[derive(RustcEncodable)]
struct PocketAuthorizeRequest<'a> {
    consumer_key: &'a str,
    code: &'a str
}

#[derive(RustcDecodable)]
struct PocketAuthorizeResponse {
    access_token: String,
    username: String
}

impl<'a> Pocket<'a> {
    pub fn new(consumer_key: &str, access_token: Option<&str>) -> Pocket<'a> {
        Pocket {
            consumer_key: consumer_key.to_string(),
            access_token: access_token.map(|v| v.to_string()),
            code: None,
            client: Client::new()
        }
    }

    pub fn access_token(&self) -> Option<&str> {
        self.access_token.as_ref().map(|v| &**v)
    }

    pub fn get_auth_url(&mut self) -> PocketResult<Url> {
        let app_json: Mime = "application/json".parse().unwrap();
        self.client.post("https://getpocket.com/v3/oauth/request")
            .header(XAccept(app_json.clone()))
            .header(ContentType(app_json.clone()))
            .body(&*json::encode(&PocketOAuthRequest {
                consumer_key: &*self.consumer_key,
                redirect_uri: "rustapi:finishauth",
                state: None
            }))
            .send().map_err(FromError::from_error)
            .and_then(|mut r| r.read_to_string().map_err(FromError::from_error))
            .and_then(|s| json::decode::<PocketOAuthResponse>(&*s).map_err(FromError::from_error))
            .and_then(|&mut: r| {
                let mut url = Url::parse("https://getpocket.com/auth/authorize").unwrap();
                url.set_query_from_pairs(vec![("request_token", &*r.code), ("redirect_uri", "rustapi:finishauth")].into_iter());
                self.code = Some(r.code);
                Ok(url)
            })
    }

    pub fn authorize(&mut self) -> PocketResult<String> {
        let app_json: Mime = "application/json".parse().unwrap();
        match {
            self.client.post("https://getpocket.com/v3/oauth/authorize")
                .header(XAccept(app_json.clone()))
                .header(ContentType(app_json.clone()))
                .body(&*json::encode(&PocketAuthorizeRequest {
                    consumer_key: &*self.consumer_key,
                    code: self.code.as_ref().map(|v| &**v).unwrap(),
                }))
                .send().map_err(FromError::from_error)
                .and_then(|mut r| r.read_to_string().map_err(FromError::from_error))
                .and_then(|s| json::decode::<PocketAuthorizeResponse>(&*s).map_err(FromError::from_error))
        } {
            Ok(r) => {
                self.access_token = Some(r.access_token);
                Ok(r.username)
            },
            Err(e) => Err(e)
        }
    }

    pub fn add(&mut self, url: &str) -> PocketResult<()> {
        Ok(())
    }
}
