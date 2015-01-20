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
use std::collections::BTreeMap;

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

#[derive(RustcEncodable)]
struct PocketAddUrlRequest<'a> {
    consumer_key: &'a str,
    access_token: &'a str,
    url: &'a str,
    title: Option<&'a str>,
    tags: Option<&'a str>,
    tweet_id: Option<&'a str>
}


#[derive(RustcDecodable, Show, PartialEq)]
pub struct ItemImage {
    pub caption: String,
    pub credit: String,
    pub height: u16, // String
    pub width: u16, // String
    pub image_id: u32, // String
    pub item_id: u32, // String
    pub src: String, // must be Url
}

#[derive(RustcDecodable, Show, PartialEq)]
pub struct ItemVideo {
    pub height: u16, // String
    pub width: u16, // String
    pub item_id: u32, // String
    pub length: u16, // String
    pub src: String, // must be Url
    //pub type: u16, // String
    pub vid: String,
    pub video_id: u32, // String
}

#[derive(RustcDecodable, Show, PartialEq)]
pub struct PocketItem {
    //pub authors: Vec<ItemAuthor>, // ???
    pub content_length: u32, // String
    pub date_published: String, // must be Tm or Timespec
    pub date_resolved: String, // must be Tm or Timespec
    pub domain_id: u32, // String
    pub encoding: String,
    pub excerpt: String,
    pub extended_item_id: u32, // String
    pub given_url: String, // must be Url?
    pub has_image: u8, // String, must be bool
    pub has_video: u8, // String, must be bool
    pub innerdomain_redirect: u8, // String, must be bool
    pub is_article: u8, // String, must be bool
    pub is_index: u8, // String, must be bool
    pub item_id: u32, // String
    pub lang: String,
    pub login_required: u8, // String, must be bool
    pub mime_type: String, // must be Option<Mime>
    pub normal_url: String, // must be Url
    pub origin_domain_id: u32, // String
    pub resolved_id: u32, // String
    pub resolved_normal_url: String, // must be Url
    pub resolved_url: String, // must be Url
    pub response_code: u16,
    pub title: String,
    pub used_fallback: u8, // String must be bool
    pub word_count: u32, // String
    pub videos: BTreeMap<usize, ItemVideo>, // must be Vec
    pub images: BTreeMap<usize, ItemImage>, // must be Vec
}

#[derive(RustcDecodable)]
struct PocketAddUrlResponse {
    item: PocketItem,
    status: u16
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

    #[inline] pub fn access_token(&self) -> Option<&str> {
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

    pub fn add(&mut self, url: &str) -> PocketResult<PocketItem> {
        let app_json: Mime = "application/json".parse().unwrap();

        self.client.post("https://getpocket.com/v3/add")
            .header(XAccept(app_json.clone()))
            .header(ContentType(app_json.clone()))
            .body(&*json::encode(&PocketAddUrlRequest {
                consumer_key: &*self.consumer_key,
                access_token: &**self.access_token.as_ref().unwrap(),
                url: url,
                title: None,
                tags: None,
                tweet_id: None
            }))
            .send().map_err(FromError::from_error)
            .and_then(|mut r| r.read_to_string().map_err(FromError::from_error))
            .and_then(|s| json::decode::<PocketAddUrlResponse>(&*s).map_err(FromError::from_error))
            .map(|v| v.item)
    }
}
