#![allow(unstable)]

extern crate hyper;
extern crate "rustc-serialize" as rustc_serialize;
extern crate url;
extern crate mime;
extern crate time;

#[cfg(test)] #[macro_use] extern crate log;

use hyper::header::{Header, HeaderFormat, ContentType};
use hyper::client::{Client, IntoUrl};
use hyper::net::HttpConnector;
use hyper::HttpError;
use hyper::header::shared::util::from_one_raw_str;
use url::Url;
use mime::Mime;
use rustc_serialize::{json, Decodable, Encodable, Decoder, Encoder};
use std::error::{FromError, Error};
use std::io::IoError;
use std::collections::BTreeMap;
use time::Timespec;

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
    url: &'a Url,
    title: Option<&'a str>,
    tags: Option<&'a str>,
    tweet_id: Option<&'a str>
}


#[derive(RustcDecodable, Show, PartialEq)]
pub struct ItemImage {
    pub item_id: u64, // String
    pub image_id: u64, // String
    pub src: Url,
    pub width: u16, // String
    pub height: u16, // String
    pub caption: String,
    pub credit: String,
}

#[derive(Show, PartialEq)]
pub struct ItemVideo {
    pub item_id: u64, // String
    pub video_id: u64, // String
    pub src: Url,
    pub width: u16, // String
    pub height: u16, // String
    pub length: usize, // String
    pub vid: String,
    pub vtype: u16,
}

impl Decodable for ItemVideo {
    fn decode<D: Decoder>(d: &mut D) -> Result<ItemVideo, D::Error> {
        d.read_struct("ItemVideo", 0, |d| Ok(ItemVideo {
            item_id: try!(d.read_struct_field("item_id", 0, |d| d.read_u64())),
            video_id: try!(d.read_struct_field("video_id", 1, |d| d.read_u64())),
            src: try!(d.read_struct_field("src", 2, Decodable::decode)),
            width: try!(d.read_struct_field("width", 3, |d| d.read_u16())),
            height: try!(d.read_struct_field("height", 4, |d| d.read_u16())),
            length: try!(d.read_struct_field("length", 5, |d| d.read_usize())),
            vid: try!(d.read_struct_field("vid", 6, |d| d.read_str())),
            vtype: try!(d.read_struct_field("type", 7, |d| d.read_u16())),
        }))
    }
}

#[derive(Show, PartialEq, Copy)]
pub enum PocketItemHas {
    No = 0,
    Yes = 1,
    Is = 2
}

impl Decodable for PocketItemHas {
    fn decode<D: Decoder>(d: &mut D) -> Result<PocketItemHas, D::Error> {
        d.read_u8().map(|v| match v {
            0 => PocketItemHas::No,
            1 => PocketItemHas::Yes,
            2 => PocketItemHas::Is,
            _ => unreachable!()
        })
    }
}

#[derive(Show, PartialEq)]
pub struct PocketAddedItem {
    pub item_id: u64, // String
    pub extended_item_id: u64, // String

    pub given_url: Url,
    pub normal_url: Url,
    pub content_length: usize, // String
    pub word_count: usize, // String
    pub encoding: String,
    pub mime_type: String, // must be Option<Mime>
    pub lang: String,
    pub title: String,
    pub excerpt: String,

    pub date_published: String, // must be Tm or Timespec
    pub date_resolved: String, // must be Tm or Timespec

    pub resolved_id: u64, // String
    pub resolved_url: Url,
    pub resolved_normal_url: Url,

    pub login_required: bool, // String
    pub response_code: u16,
    pub used_fallback: bool, // String

    pub domain_id: u64, // String
    pub origin_domain_id: u64, // String
    pub innerdomain_redirect: bool,

    pub is_index: bool, // String
    pub is_article: bool, // String
    pub has_image: PocketItemHas, // String
    pub has_video: PocketItemHas, // String

    //pub tags: Vec<ItemTag>, // ???
    //pub authors: Vec<ItemAuthor>, // ???
    pub videos: Vec<ItemVideo>, // encoded as object with integer indices
    pub images: Vec<ItemImage>, // if present, as empty array otherwise
}

impl Decodable for PocketAddedItem {
    fn decode<D: Decoder>(d: &mut D) -> Result<PocketAddedItem, D::Error> {
        d.read_struct("PocketAddedItem", 28, |d| Ok(PocketAddedItem {
            item_id: try!(d.read_struct_field("item_id", 0, |d| d.read_u64())),
            extended_item_id: try!(d.read_struct_field("extended_item_id", 1, |d| d.read_u64())),

            given_url: try!(d.read_struct_field("given_url", 2, Decodable::decode)),
            normal_url: try!(d.read_struct_field("normal_url", 3, Decodable::decode)),
            content_length: try!(d.read_struct_field("content_length", 4, |d| d.read_usize())),
            word_count: try!(d.read_struct_field("word_count", 5, |d| d.read_usize())),
            encoding: try!(d.read_struct_field("encoding", 6, |d| d.read_str())),
            mime_type: try!(d.read_struct_field("mime_type", 7, |d| d.read_str())),
            lang: try!(d.read_struct_field("lang", 8, |d| d.read_str())),
            title: try!(d.read_struct_field("title", 9, |d| d.read_str())),
            excerpt: try!(d.read_struct_field("excerpt", 10, |d| d.read_str())),

            date_published: try!(d.read_struct_field("date_published", 11, |d| d.read_str())),
            date_resolved: try!(d.read_struct_field("date_resolved", 12, |d| d.read_str())),

            resolved_id: try!(d.read_struct_field("resolved_id", 13, |d| d.read_u64())),
            resolved_url: try!(d.read_struct_field("resolved_url", 14, Decodable::decode)),
            resolved_normal_url: try!(d.read_struct_field("resolved_normal_url", 15, Decodable::decode)),

            login_required: try!(d.read_struct_field("login_required", 16, |d| d.read_u8().map(|v| v != 0))),
            response_code: try!(d.read_struct_field("response_code", 17, |d| d.read_u16())),
            used_fallback: try!(d.read_struct_field("used_fallback", 18, |d| d.read_u8().map(|v| v != 0))),

            domain_id: try!(d.read_struct_field("domain_id", 19, |d| d.read_u64())),
            origin_domain_id: try!(d.read_struct_field("origin_domain_id", 20, |d| d.read_u64())),
            innerdomain_redirect: try!(d.read_struct_field("innerdomain_redirect", 21, |d| d.read_u8().map(|v| v != 0))),

            is_index: try!(d.read_struct_field("is_index", 22, |d| d.read_u8().map(|v| v != 0))),
            is_article: try!(d.read_struct_field("is_article", 23, |d| d.read_u8().map(|v| v != 0))),
            has_image: try!(d.read_struct_field("has_image", 24, Decodable::decode)),
            has_video: try!(d.read_struct_field("has_video", 25, Decodable::decode)),

            videos: try!(d.read_struct_field("videos", 26, |d| d.read_map(|d, s|
                Ok(range(0, s).flat_map(|i| d.read_map_elt_key(i, |d| d.read_str()).and_then(|_| d.read_map_elt_val(i, Decodable::decode)).into_iter()).collect())
            ))),
            images: try!(d.read_struct_field("images", 27, |d| d.read_map(|d, s|
                Ok(range(0, s).flat_map(|i| d.read_map_elt_key(i, |d| d.read_str()).and_then(|_| d.read_map_elt_val(i, Decodable::decode)).into_iter()).collect())
            )))
        }))
    }
}

#[derive(RustcDecodable)]
struct PocketAddResponse {
    item: PocketAddedItem,
    status: u16
}

pub struct PocketGetRequest<'a> {
    consumer_key: String,
    access_token: String,

    search: Option<&'a str>,
    domain: Option<&'a str>,

    tag: Option<PocketGetTag<'a>>,
    state: Option<PocketGetState>,
    content_type: Option<PocketGetType>,
    detail_type: Option<PocketGetDetail>,
    favorite: Option<bool>,
    since: Option<Timespec>,

    sort: Option<PocketGetSort>,
    count: Option<usize>,
    offset: Option<usize>
}

impl<'a> PocketGetRequest<'a> {
    fn new<'b>(consumer_key: &'b str, access_token: &'b str) -> PocketGetRequest<'a> {
        PocketGetRequest {
            consumer_key: consumer_key.to_string(),
            access_token: access_token.to_string(),
            search: None,
            domain: None,
            tag: None,
            state: None,
            content_type: None,
            detail_type: None,
            favorite: None,
            since: None,
            sort: None,
            count: None,
            offset: None
        }
    }

    pub fn search(mut self, search: &'a str) -> PocketGetRequest<'a> {
        self.search = Some(search);
        self
    }

    pub fn domain(mut self, domain: &'a str) -> PocketGetRequest<'a> {
        self.domain = Some(domain);
        self
    }

    pub fn tag(mut self, tag: PocketGetTag<'a>) -> PocketGetRequest<'a> {
        self.tag = Some(tag);
        self
    }

    pub fn state(mut self, state: PocketGetState) -> PocketGetRequest<'a> {
        self.state = Some(state);
        self
    }

    pub fn content_type(mut self, content_type: PocketGetType) -> PocketGetRequest<'a> {
        self.content_type = Some(content_type);
        self
    }

    pub fn detail_type(mut self, detail_type: PocketGetDetail) -> PocketGetRequest<'a> {
        self.detail_type = Some(detail_type);
        self
    }

    pub fn complete(self) -> PocketGetRequest<'a> {
        self.detail_type(PocketGetDetail::Complete)
    }

    pub fn simple(self) -> PocketGetRequest<'a> {
        self.detail_type(PocketGetDetail::Simple)
    }

    pub fn archived(self) -> PocketGetRequest<'a> {
        self.state(PocketGetState::Archive)
    }

    pub fn unread(self) -> PocketGetRequest<'a> {
        self.state(PocketGetState::Unread)
    }

    pub fn articles(self) -> PocketGetRequest<'a> {
        self.content_type(PocketGetType::Article)
    }

    pub fn videos(self) -> PocketGetRequest<'a> {
        self.content_type(PocketGetType::Video)
    }

    pub fn images(self) -> PocketGetRequest<'a> {
        self.content_type(PocketGetType::Image)
    }

    pub fn favorite(mut self, fav: bool) -> PocketGetRequest<'a> {
        self.favorite = Some(fav);
        self
    }

    pub fn since(mut self, since: Timespec) -> PocketGetRequest<'a> {
        self.since = Some(since);
        self
    }

    pub fn sort(mut self, sort: PocketGetSort) -> PocketGetRequest<'a> {
        self.sort = Some(sort);
        self
    }

    pub fn sort_by_newest(self) -> PocketGetRequest<'a> {
        self.sort(PocketGetSort::Newest)
    }

    pub fn sort_by_oldest(self) -> PocketGetRequest<'a> {
        self.sort(PocketGetSort::Oldest)
    }

    pub fn sort_by_title(self) -> PocketGetRequest<'a> {
        self.sort(PocketGetSort::Title)
    }

    pub fn sort_by_site(self) -> PocketGetRequest<'a> {
        self.sort(PocketGetSort::Site)
    }

    pub fn offset(mut self, offset: usize) -> PocketGetRequest<'a> {
        self.offset = Some(offset);
        self
    }

    pub fn count(mut self, count: usize) -> PocketGetRequest<'a> {
        self.count = Some(count);
        self
    }

    pub fn slice(self, offset: usize, count: usize) -> PocketGetRequest<'a> {
        self.offset(offset).count(count)
    }
}

impl<'a> Encodable for PocketGetRequest<'a> {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
        e.emit_struct("PocketGetRequest", 0, |e|
            e.emit_struct_field("consumer_key", 0, |e| e.emit_str(&*self.consumer_key)).and_then(|_|
            e.emit_struct_field("access_token", 1, |e| e.emit_str(&*self.access_token))).and_then(|_|

            e.emit_struct_field("search", 2, |e| e.emit_option(|e| self.search.map(|v|
                e.emit_option_some(|e| e.emit_str(v))).unwrap_or_else(|| e.emit_option_none())
            ))).and_then(|_|
            e.emit_struct_field("domain", 3, |e| e.emit_option(|e| self.domain.map(|v|
                e.emit_option_some(|e| e.emit_str(v))).unwrap_or_else(|| e.emit_option_none())
            ))).and_then(|_|

            e.emit_struct_field("tag", 4, |e| e.emit_option(|e|
                self.tag.as_ref().map(|v| e.emit_option_some(|e| v.encode(e)))
                    .unwrap_or_else(|| e.emit_option_none())))).and_then(|_|

            e.emit_struct_field("state", 5, |e| e.emit_option(|e|
                self.state.as_ref().map(|v| e.emit_option_some(|e| v.encode(e)))
                    .unwrap_or_else(|| e.emit_option_none())))).and_then(|_|

            e.emit_struct_field("contentType", 6, |e| e.emit_option(|e|
                self.content_type.as_ref().map(|v| e.emit_option_some(|e| v.encode(e)))
                    .unwrap_or_else(|| e.emit_option_none())))).and_then(|_|

            e.emit_struct_field("detailType", 7, |e| e.emit_option(|e|
                self.detail_type.as_ref().map(|v| e.emit_option_some(|e| v.encode(e)))
                    .unwrap_or_else(|| e.emit_option_none())))).and_then(|_|

            e.emit_struct_field("favorite", 8, |e| e.emit_option(|e| self.favorite.map(|v|
                e.emit_option_some(|e| e.emit_bool(v))).unwrap_or_else(|| e.emit_option_none())
            ))).and_then(|_|

            e.emit_struct_field("since", 9, |e| e.emit_option(|e| self.since.map(|v|
                e.emit_option_some(|e| e.emit_u64(v.sec as u64))).unwrap_or_else(|| e.emit_option_none())
            ))).and_then(|_|

            e.emit_struct_field("sort", 10, |e| e.emit_option(|e|
                self.sort.as_ref().map(|v| e.emit_option_some(|e| v.encode(e)))
                    .unwrap_or_else(|| e.emit_option_none())))).and_then(|_|

            e.emit_struct_field("count", 11, |e| e.emit_option(|e| self.count.map(|v|
                e.emit_option_some(|e| e.emit_usize(v))).unwrap_or_else(|| e.emit_option_none())
            ))).and_then(|_|

            e.emit_struct_field("offset", 12, |e| e.emit_option(|e| self.offset.map(|v|
                e.emit_option_some(|e| e.emit_usize(v))).unwrap_or_else(|| e.emit_option_none())
            )))
        )
    }
}

#[derive(Show, Copy)]
pub enum PocketGetDetail {
    Simple,
    Complete
}
impl Encodable for PocketGetDetail {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
        match *self {
            PocketGetDetail::Simple => e.emit_str("simple"),
            PocketGetDetail::Complete => e.emit_str("complete"),
        }
    }
}

#[derive(Show, Copy)]
pub enum PocketGetSort {
    Newest,
    Oldest,
    Title,
    Site
}

impl Encodable for PocketGetSort {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
        match *self {
            PocketGetSort::Newest => e.emit_str("newest"),
            PocketGetSort::Oldest => e.emit_str("oldest"),
            PocketGetSort::Title => e.emit_str("title"),
            PocketGetSort::Site => e.emit_str("site"),
        }
    }
}

#[derive(Show, Copy)]
pub enum PocketGetState {
    Unread,
    Archive,
    All
}

impl Encodable for PocketGetState {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
        match *self {
            PocketGetState::Unread => e.emit_str("unread"),
            PocketGetState::Archive => e.emit_str("archive"),
            PocketGetState::All => e.emit_str("all"),
        }
    }
}

#[derive(Show)]
pub enum PocketGetTag<'a> {
    Untagged,
    Tagged(&'a str)
}

impl<'a> Encodable for PocketGetTag<'a> {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
        match *self {
            PocketGetTag::Untagged => e.emit_str("_untagged_"),
            PocketGetTag::Tagged(ref s) => e.emit_str(&**s),
        }
    }
}

#[derive(Show, Copy)]
pub enum PocketGetType {
    Article,
    Video,
    Image
}

impl Encodable for PocketGetType {
    fn encode<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
        match *self {
            PocketGetType::Article => e.emit_str("article"),
            PocketGetType::Video => e.emit_str("video"),
            PocketGetType::Image => e.emit_str("image"),
        }
    }
}

#[derive(Show)]
struct PocketGetResponse {
    list: Vec<PocketItem>, // must be Vec
    status: u16,
    complete: bool, // must be bool
    error: Option<String>,
    //search_meta: PocketSearchMeta,
    since: Timespec,
}

impl Decodable for PocketGetResponse {
    fn decode<D: Decoder>(d: &mut D) -> Result<PocketGetResponse, D::Error> {
        d.read_struct("PocketGetResponse", 5, |d| Ok(PocketGetResponse {
            list: try!(d.read_struct_field("list", 0, |d| d.read_map(|d, s|
                Ok(range(0, s).flat_map(|i|
                    d.read_map_elt_key(i, |d| d.read_str()).and_then(|_|
                    d.read_map_elt_val(i, Decodable::decode)).into_iter())
                .collect())
            ))),
            status: try!(d.read_struct_field("status", 1, |d| d.read_u16())),
            complete: try!(d.read_struct_field("complete", 2, |d| d.read_u8().map(|v| v != 0))),
            error: try!(d.read_struct_field("error", 3, |d| d.read_option(|d, b| if b { d.read_str().map(Some) } else { Ok(None) }))),
            since: try!(d.read_struct_field("since", 4, |d| d.read_u64().map(|v| Timespec::new(v as i64, 0))))
        }))
    }
}

#[derive(Show, PartialEq, Copy)]
pub enum PocketItemStatus {
    Normal = 0,
    Archived = 1,
    Deleted = 2
}

impl Decodable for PocketItemStatus {
    fn decode<D: Decoder>(d: &mut D) -> Result<PocketItemStatus, D::Error> {
        d.read_u8().map(|v| match v {
            0 => PocketItemStatus::Normal,
            1 => PocketItemStatus::Archived,
            2 => PocketItemStatus::Deleted,
            _ => unreachable!()
        })
    }
}

// See also PocketAddedItem
#[derive(Show, PartialEq)]
pub struct PocketItem {
    pub item_id: u64,

    pub given_url: Url,
    pub given_title: String,

    pub word_count: usize,
    pub excerpt: String,

    pub time_added: Timespec,
    pub time_read: Timespec,
    pub time_updated: Timespec,
    pub time_favorited: Timespec,

    pub favorite: bool,

    pub is_index: bool,
    pub is_article: bool,
    pub has_image: PocketItemHas,
    pub has_video: PocketItemHas,

    pub resolved_id: u64,
    pub resolved_title: String,
    pub resolved_url: Url,

    pub sort_id: usize,

    pub status: PocketItemStatus,
    pub images: Option<Vec<ItemImage>>,
    pub videos: Option<Vec<ItemVideo>>,
}

impl Decodable for PocketItem {
    fn decode<D: Decoder>(d: &mut D) -> Result<PocketItem, D::Error> {
        d.read_struct("PocketItem", 21, |d| Ok(PocketItem {
            item_id: try!(d.read_struct_field("item_id", 0, |d| d.read_u64())),

            given_url: try!(d.read_struct_field("given_url", 1, Decodable::decode)),
            given_title: try!(d.read_struct_field("given_title", 2, |d| d.read_str())),

            word_count: try!(d.read_struct_field("word_count", 3, |d| d.read_usize())),
            excerpt: try!(d.read_struct_field("excerpt", 4, |d| d.read_str())),

            time_added: try!(d.read_struct_field("time_added", 5, |d| d.read_u64().map(|v| Timespec::new(v as i64, 0)))),
            time_read: try!(d.read_struct_field("time_read", 6, |d| d.read_u64().map(|v| Timespec::new(v as i64, 0)))),
            time_updated: try!(d.read_struct_field("time_updated", 7, |d| d.read_u64().map(|v| Timespec::new(v as i64, 0)))),
            time_favorited: try!(d.read_struct_field("time_favorited", 8, |d| d.read_u64().map(|v| Timespec::new(v as i64, 0)))),

            favorite: try!(d.read_struct_field("favorite", 9, |d| d.read_u8().map(|v| v != 0))),
            is_index: try!(d.read_struct_field("is_index", 10, |d| d.read_u8().map(|v| v != 0))),
            is_article: try!(d.read_struct_field("is_article", 11, |d| d.read_u8().map(|v| v != 0))),
            has_image: try!(d.read_struct_field("has_image", 12, Decodable::decode)),
            has_video: try!(d.read_struct_field("has_video", 13, Decodable::decode)),

            resolved_id: try!(d.read_struct_field("resolved_id", 14, |d| d.read_u64())),
            resolved_title: try!(d.read_struct_field("resolved_title", 15, |d| d.read_str())),
            resolved_url: try!(d.read_struct_field("resolved_url", 16, Decodable::decode)),

            sort_id: try!(d.read_struct_field("sort_id", 17, |d| d.read_usize())),
            status: try!(d.read_struct_field("status", 18, Decodable::decode)),

            videos: try!(d.read_struct_field("videos", 19, |d| d.read_option(|d, b| if b {
                d.read_map(|d, s| Ok(range(0, s).flat_map(|i| d.read_map_elt_val(i, Decodable::decode).into_iter()).collect())).map(Some)
            } else {
                Ok(None)
            }))),
            images: try!(d.read_struct_field("images", 20, |d| d.read_option(|d, b| if b {
                d.read_map(|d, s| Ok(range(0, s).flat_map(|i| d.read_map_elt_val(i, Decodable::decode).into_iter()).collect())).map(Some)
            } else {
                Ok(None)
            })))
        }))
    }
}

#[derive(RustcEncodable)]
struct PocketAddAction<'a> {
    item_id: Option<u64>,
    ref_id: Option<&'a str>,
    tags: Option<&'a str>,
    time: Option<u64>,
    title: Option<&'a str>,
    url: Option<&'a Url>
}

#[derive(RustcEncodable)]
struct PocketArchiveAction {
    item_id: u64,
    time: Option<u64>,
}

#[derive(RustcEncodable)]
struct PocketReaddAction {
    item_id: u64,
    time: Option<u64>,
}

#[derive(RustcEncodable)]
struct PocketFavoriteAction {
    item_id: u64,
    time: Option<u64>,
}

#[derive(RustcEncodable)]
struct PocketUnfavoriteAction {
    item_id: u64,
    time: Option<u64>,
}

#[derive(RustcEncodable)]
struct PocketDeleteAction {
    item_id: u64,
    time: Option<u64>,
}

#[derive(RustcEncodable)]
struct PocketTagsAddAction<'a> {
    item_id: u64,
    tags: &'a str,
    time: Option<u64>,
}

#[derive(RustcEncodable)]
struct PocketTagsReplaceAction<'a> {
    item_id: u64,
    tags: &'a str,
    time: Option<u64>,
}

#[derive(RustcEncodable)]
struct PocketTagsClearAction {
    item_id: u64,
    time: Option<u64>,
}

#[derive(RustcEncodable)]
struct PocketTagRenameAction<'a> {
    item_id: u64,
    old_tag: &'a str,
    new_tag: &'a str,
    time: Option<u64>,
}

trait PocketAction : Encodable {
    fn name(marker: Option<Self>) -> &'static str;
}

#[derive(RustcEncodable)]
struct PocketSendRequest<'a> {
    consumer_key: &'a str,
    access_token: &'a str,
    //actions: &'a [&'a PocketAction]
}

#[derive(RustcDecodable)]
struct PocketSendResponse {
    status: u16,
    action_results: Vec<bool>
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

    pub fn add<T: IntoUrl>(&mut self, url: T) -> PocketResult<PocketAddedItem> {
        let request = json::encode(&PocketAddRequest {
            consumer_key: &*self.consumer_key,
            access_token: &**self.access_token.as_ref().unwrap(),
            url: &url.into_url().unwrap(),
            title: None,
            tags: None,
            tweet_id: None
        });

        self.request("https://getpocket.com/v3/add", &*request)
            .map(|v: PocketAddResponse| v.item)
    }

    pub fn get(&mut self, filter: &PocketGetRequest) -> PocketResult<Vec<PocketItem>> {
        let request = json::encode(&filter);

        self.request("https://getpocket.com/v3/get", &*request)
            .map(|v: PocketGetResponse| v.list)
    }

    pub fn filter<'s, 'b>(&'s self) -> PocketGetRequest<'b> {
        PocketGetRequest::new(&*self.consumer_key, self.access_token.as_ref().map(|a| &**a).unwrap())
    }
}
