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
use std::collections::BTreeMap;

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
            PocketError::Proto(ref code, ref msg) => Some(format!("{} (code {})", msg, code))
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
        std::fmt::String::fmt(&self.0, fmt)
    }
}

impl Header for XErrorCode {
    #[allow(unused_variables)]
    fn header_name(marker: Option<Self>) -> &'static str {
        "X-Error-Code"
    }

    fn parse_header(raw: &[Vec<u8>]) -> Option<XErrorCode> {
        from_one_raw_str(raw).map(|code| XErrorCode(code))
    }
}

impl HeaderFormat for XErrorCode {
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
struct PocketAddRequest<'a> {
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
    pub image_id: u64, // String
    pub item_id: u64, // String
    pub src: String, // must be Url
}

#[derive(RustcDecodable, Show, PartialEq)]
pub struct ItemVideo {
    pub height: u16, // String
    pub width: u16, // String
    pub item_id: u64, // String
    pub length: usize, // String
    pub src: String, // must be Url
    //pub type: u16, // String
    pub vid: String,
    pub video_id: u64, // String
}

#[derive(RustcDecodable, Show, PartialEq)]
pub struct PocketAddedItem {
    pub content_length: usize, // String
    pub date_published: String, // must be Tm or Timespec
    pub date_resolved: String, // must be Tm or Timespec
    pub domain_id: u32, // String
    pub encoding: String,
    pub excerpt: String,
    pub extended_item_id: u64, // String
    pub given_url: String, // must be Url?
    pub has_image: u8, // String, must be enum PocketItemHas { DontHas = 0, Has = 1, Is = 2 }
    pub has_video: u8, // String, must be enum PocketItemHas
    pub innerdomain_redirect: u8, // String, must be bool
    pub is_article: u8, // String, must be bool
    pub is_index: u8, // String, must be bool
    pub item_id: u64, // String
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
    //pub favorite: u8, // String must be bool
    //pub status: u8, // String must be enum PocketItemStatus { Normal = 0, Archived = 1, Deleted = 2 }
    //pub tags: Vec<ItemTag>, // ???
    //pub authors: Vec<ItemAuthor>, // ???
    //pub videos: Vec<ItemVideo>, // encoded as object with integer indices
    //pub images: Vec<ItemImage>, // if present, as empty array otherwise
}

#[derive(RustcDecodable)]
struct PocketAddResponse {
    item: PocketAddedItem,
    status: u16
}

#[derive(RustcEncodable)]
#[allow(non_snake_case)]
struct PocketGetRequest<'a> {
    consumer_key: &'a str,
    access_token: &'a str,
    state: Option<&'a str>, // must be enum PocketItemState { unread, archive, all }
    favorite: Option<u8>,  // must be bool
    tag: Option<&'a str>,   // should be enum PocketTag { Untagged, Tagged(String) }
    contentType: Option<&'a str>, // must be enum PocketItemType { article, video, image }
    sort: Option<&'a str>, // must be enum PocketSort { newest, oldest, title, site }
    detailType: Option<&'a str>, // must be enum PocketDetailType { simple, complete } or just a bool
    search: Option<&'a str>,
    domain: Option<&'a str>,
    since: Option<u64>, // must be Timespec or Tm
    count: Option<usize>,
    offset: Option<usize>
}

#[derive(RustcDecodable)]
struct PocketGetResponse {
    list: BTreeMap<String, PocketItem>, // must be Vec
    status: u16,
    complete: u8, // must be bool
    error: Option<String>,
    //search_meta: PocketSearchMeta,
    since: u64, // must be Timespec or Tm
}

// See also PocketAddedItem
#[derive(RustcDecodable, Show, PartialEq)]
pub struct PocketItem {
    pub excerpt: String,
    pub favorite: u8, // bool
    pub given_title: String,
    pub given_url: String, // Url
    pub has_image: u8, // enum PocketItemHas
    pub has_video: u8,
    pub is_article: u8, // bool
    pub is_index: u8, // bool
    pub item_id: u64,
    pub resolved_id: u64,
    pub resolved_title: String,
    pub resolved_url: String, // Url
    pub sort_id: usize,
    pub status: u8, // enum PocketItemStatus
    pub time_added: u64, // Tm/Timespec
    pub time_favorited: u64, // Tm/Timespec
    pub time_read: u64, // Tm/Timespec
    pub time_updated: u64, // Tm/Timespec
    pub word_count: u32,
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
            .and_then(|mut r| match r.headers.get::<XErrorCode>().map(|v| v.0) {
                None => r.read_to_string().map_err(FromError::from_error),
                Some(code) => Err(PocketError::Proto(code, r.headers.get::<XError>().map(|v| &*v.0).unwrap_or("unknown protocol error").to_string())),
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

    pub fn add(&mut self, url: &str) -> PocketResult<PocketAddedItem> {
        let request = json::encode(&PocketAddRequest {
            consumer_key: &*self.consumer_key,
            access_token: &**self.access_token.as_ref().unwrap(),
            url: url,
            title: None,
            tags: None,
            tweet_id: None
        });

        self.request("https://getpocket.com/v3/add", &*request)
            .map(|v: PocketAddResponse| v.item)
    }

    pub fn get(&mut self) -> PocketResult<Vec<PocketItem>> {
        let request = json::encode(&PocketGetRequest {
            consumer_key: &*self.consumer_key,
            access_token: &**self.access_token.as_ref().unwrap(),
            state: None,
            favorite: None,
            tag: None,
            contentType: None,
            sort: None,
            detailType: None,
            search: None,
            domain: None,
            since: None,
            count: None,
            offset: None
        });

        self.request("https://getpocket.com/v3/get", &*request)
            .map(|v: PocketGetResponse| v.list.into_iter().map(|(_, v)| v).collect())
    }
}
