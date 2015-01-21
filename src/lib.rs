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
use rustc_serialize::{json, Decodable, Encodable};
use std::error::{FromError, Error};
use std::io::IoError;

#[derive(Show)]
pub enum PocketError {
    Http(HttpError),
    Json(json::DecoderError),
    Proto(u16, String)
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
            PocketError::Proto(..) => "protocol error"
        }
    }

    fn detail(&self) -> Option<String> {
        match *self {
            PocketError::Http(ref e) => e.detail(),
            PocketError::Json(ref e) => e.detail(),
            PocketError::Proto(ref code, ref msg) => format!("{} (code {})", msg, code)
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            PocketError::Http(ref e) => Some(e),
            PocketError::Json(ref e) => Some(e),
            PocketError::Proto(..) => None
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

#[derive(Clone)]
struct XError(String);
#[derive(Clone)]
struct XErrorCode(u16);

impl Header for XError {
    #[allow(unused_variables)]
    fn header_name(marker: Option<Self>) -> &'static str {
        "X-Error"
    }

    fn parse_header(raw: &[Vec<u8>]) -> Option<XError> {
        from_one_raw_str(raw).map(|error| XError(error))
    }
}

impl HeaderFormat for XError {
    fn fmt_header(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(fmt)
    }
}

impl Header for XErrorCode {
    #[allow(unused_variables)]
    fn header_name(marker: Option<Self>) -> &'static str {
        "X-Error"
    }

    fn parse_header(raw: &[Vec<u8>]) -> Option<XErrorCode> {
        from_one_raw_str(raw).map(|code| XErrorCode(code))
    }
}

impl HeaderFormat for XErrorCode {
    fn fmt_header(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(fmt)
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
    //pub authors: Vec<ItemAuthor>, // ???
    //pub videos: Vec<ItemVideo>, // encoded as object with integer indices
    //pub images: Vec<ItemImage>, // if present, as empty array otherwise
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

    fn request<Resp: Decodable>(&mut self, url: &str, data: &str) -> PocketResult<Resp> {
        let app_json: Mime = "application/json".parse().unwrap();
        self.client.post(url)
            .header(XAccept(app_json.clone()))
            .header(ContentType(app_json.clone()))
            .body(data)
            .send().map_err(FromError::from_error)
            .and_then(|mut r| {
                match (r.headers.get::<XErrorCode>(), r.headers.get::<XError>()) {
                    (Some(XErrorCode(code)), Some(XError(error))) => Err(PocketError::Proto(code, error)),
                    (None, None) => r.read_to_string().map_err(FromError::from_error),

                    (Some(XErrorCode(code)), None) => Err(PocketError::Proto(code, "unknown protocol error".to_string())),
                    (None, Some(XError(error))) => Err(PocketError::Proto(0, error)),
                }
            })
            .and_then(|s| json::decode::<Resp>(&*s).map_err(FromError::from_error))
    }

    pub fn get_auth_url(&mut self) -> PocketResult<Url> {
        let request = json::encode(&PocketOAuthRequest {
            consumer_key: &*self.consumer_key,
            redirect_uri: "rustapi:finishauth",
            state: None
        });

        self.request("https://getpocket.com/v3/oauth/request", &*request)
            .and_then(|&mut: r: PocketOAuthResponse| {
                let mut url = Url::parse("https://getpocket.com/auth/authorize").unwrap();
                url.set_query_from_pairs(vec![("request_token", &*r.code), ("redirect_uri", "rustapi:finishauth")].into_iter());
                self.code = Some(r.code);
                Ok(url)
            })
    }

    pub fn authorize(&mut self) -> PocketResult<String> {
        let request = json::encode(&PocketAuthorizeRequest {
            consumer_key: &*self.consumer_key,
            code: self.code.as_ref().map(|v| &**v).unwrap()
        });

        match self.request("https://getpocket.com/v3/oauth/authorize", &*request)
        {
            Ok(r @ PocketAuthorizeResponse {..}) => {
                self.access_token = Some(r.access_token);
                Ok(r.username)
            },
            Err(e) => Err(e)
        }
    }

    pub fn add(&mut self, url: &str) -> PocketResult<PocketItem> {
        let request = json::encode(&PocketAddUrlRequest {
            consumer_key: &*self.consumer_key,
            access_token: &**self.access_token.as_ref().unwrap(),
            url: url,
            title: None,
            tags: None,
            tweet_id: None
        });

        self.request("https://getpocket.com/v3/add", &*request)
            .map(|v: PocketAddUrlResponse| v.item)
    }
}
