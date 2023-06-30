use activitypub_federation::{
  activity_queue::send_activity,
  config::{Data, FederationConfig},
  traits::{ActivityHandler, Actor, Object},
};
use actix_web::{
  get,
  http::header::ContentType,
  web,
  App,
  HttpResponse,
  HttpServer,
  Responder,
  ResponseError,
};
use anyhow::{Context, Result};
use async_stream::stream;
use async_trait::async_trait;
use clap::Parser;
use futures_core::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Deserializer, Value};
use std::{collections::HashMap, fmt::Display, path::Path, pin::pin, str::FromStr, time::Instant};
use time::format_description::well_known::Rfc3339;
use tokio_stream::StreamExt;
use url::Url;
const SERVER_URL: &str = "http://reddit.com.localhost:5313";

#[derive(Debug, Parser)]
struct Options {
  /// at which host and port we listen
  #[arg(default_value_t=Url::parse("http://reddit.com.localhost:5313").unwrap())]
  local_server: Url,
  /// at which host and port we send federation events to
  #[arg(long)]
  remote_server: Url,
  /// the root directory of the reddit dump. this directory should contain the files `comments/RC_2022-12.zst` and `submissions/RS_2022-12.zst`
  #[arg(long)]
  input_dir: String,
  /// when this number of events have been sent, stop
  #[arg(default_value_t = 10000, long)]
  limit: i64,
  /// skip this number of entries from the input files to make the ratio from comment to post more realistic (because many comments are for older posts we don't have)
  #[arg(default_value_t = 100000, long)]
  skip: i64,
}
#[derive(Debug, Deserialize, Clone)]
struct Submission {
  url: String, // "https://i.redd.it/t0zqz3vuw43a1.jpg",
  //crosspost_parent: Option<String>, // "t3_z8rznr",
  //crosspost_parent_list: Option<Vec<Submission>>,
  author: String,    // "icepirate87",
  permalink: String, // "/r/u_icepirate87/comments/z97uwu/meirl_i_love_subtitles/",
  created_utc: f64,  // unix seconds 1669852800,
  id: String,        // "z97uwu",
  subreddit: String, // u_icepirate87
  //subreddit_name_prefixed: String, // "u/icepirate87",
  title: String,
  selftext: String,
  /*
  // other properties we don't need rn
      "all_awardings": [],
      "allow_live_comments": false,
      "archived": false,

      "author_created_utc": 1644845414,
      "author_flair_background_color": null,
      "author_flair_css_class": null,
      "author_flair_richtext": [],
      "author_flair_template_id": null,
      "author_flair_text": null,
      "author_flair_text_color": null,
      "author_flair_type": "text",
      "author_fullname": "t2_jorem03w",
      "author_patreon_flair": false,
      "author_premium": false,
      "awarders": [],
      "banned_by": null,
      "can_gild": true,
      "can_mod_post": false,
      "category": null,
      "content_categories": null,
      "contest_mode": false,
      "discussion_type": null,
      "distinguished": null,
      "domain": "i.redd.it",
      "edited": false,
      "gilded": 0,
      "gildings": {},
      "hidden": false,
      "hide_score": false,
      "is_created_from_ads_ui": false,
      "is_crosspostable": true,
      "is_meta": false,
      "is_original_content": false,
      "is_reddit_media_domain": true,
      "is_robot_indexable": true,
      "is_self": false,
      "is_video": false,
      "link_flair_background_color": "",
      "link_flair_css_class": null,
      "link_flair_richtext": [],
      "link_flair_text": null,
      "link_flair_text_color": "dark",
      "link_flair_type": "text",
      "locked": false,
      "media": null,
      "media_embed": {},
      "media_only": false,
      "name": "t3_z97uwu",
      "no_follow": true,
      "num_comments": 0,
      "num_crossposts": 0,
      "over_18": true,
      "parent_whitelist_status": null,
      "pinned": false,
      "post_hint": "image",
      "preview": {
        "enabled": true,
        "images": [...]
      },
      "pwls": null,
      "quarantine": false,
      "removed_by": null,
      "removed_by_category": null,
      "retrieved_on": 1673199077,
      "score": 1,
      "secure_media": null,
      "secure_media_embed": {},
      "selftext": "",
      "send_replies": false,
      "spoiler": false,
      "stickied": false,
      "subreddit_id": "t5_5uo70f",
      "subreddit_subscribers": 0,
      "subreddit_type": "user",
      "suggested_sort": "qa",
      "thumbnail": "nsfw",
      "thumbnail_height": 140,
      "thumbnail_width": 140,
      "top_awarded_type": null,
      "total_awards_received": 0,
      "treatment_tags": [],
      "upvote_ratio": 1,
      "url": "https://i.redd.it/t0zqz3vuw43a1.jpg",
      "url_overridden_by_dest": "https://i.redd.it/t0zqz3vuw43a1.jpg",
      "view_count": null,
      "whitelist_status": null,
      "wls": null
    }
  */
}

#[derive(Deserialize, Debug, Clone)]
struct Comment {
  body: String, // "This was mine too! Everyone is always suggesting Le Bouchon",
  author: String,
  id: String, // "iyfeo5a",
  permalink: String,
  //link_id: String,                 // "t3_z8ze4k",
  subreddit: String, // "chicagofood",
  //subreddit_name_prefixed: String, // "r/chicagofood",
  parent_id: String, // "t1_iyecr2i",
  //score: i64,
  // seconds. 1669852800
  created_utc: f64,
  /*
  // other properties we don't need rn
  all_awardings: [],
  archived: false,
  associated_award: null,
  author_created_utc: 1616531052,
  author_flair_background_color: null,
  author_flair_css_class: null,
  author_flair_richtext: [],
  author_flair_template_id: null,
  author_flair_text: null,
  author_flair_text_color: null,
  author_flair_type: "text",
  author_fullname: "t2_b3gouosd",
  author_patreon_flair: false,
  author_premium: false,
  can_gild: true,
  collapsed: false,
  collapsed_because_crowd_control: null,
  collapsed_reason: null,
  collapsed_reason_code: null,
  comment_type: null,
  controversiality: 0,
  distinguished: null,
  edited: false,
  gilded: 0,
  gildings: {},
  is_submitter: false,
  locked: false,
  name: "t1_iyfeo5a",
  no_follow: true,
  retrieved_on: 1671055210,
  score_hidden: false,
  send_replies: true,
  stickied: false,
  subreddit_id: "t5_2s9qq",
  subreddit_type: "public",
  top_awarded_type: null,
  total_awards_received: 0,
  treatment_tags: [],
  unrepliable_reason: null
  */
}
async fn import_reddit_dump<T>(source_file: &Path) -> Result<impl Stream<Item = Result<T>>>
where
  T: for<'a> Deserialize<'a> + Send + 'static,
{
  let mut comments_stream = zstd::stream::Decoder::new(std::fs::File::open(source_file)?)?;
  comments_stream.window_log_max(31)?;
  let mut de = Deserializer::from_reader(comments_stream).into_iter::<T>();

  Ok(stream! {
    loop {
      let (next, den) = tokio::task::spawn_blocking(|| {
        (de.next(), de)
      }).await.context("spawning error")?;
      de = den;
      let Some(next) = next else {
        return;
      };
      yield next.context("decoding error");
    }
  })
}

enum SubmissionOrComment {
  Submission(Submission),
  Comment(Comment),
}
/// load the comments and posts file and intermingle them ordered by time
async fn import_merged_reddit_dump(
  source_dir: &Path,
  month: &str,
) -> Result<impl Stream<Item = Result<SubmissionOrComment>>> {
  let mut comment_stream =
    Box::pin(import_reddit_dump(&source_dir.join(format!("comments/RC_{month}.zst"))).await?);
  let mut post_stream =
    Box::pin(import_reddit_dump(&source_dir.join(format!("submissions/RS_{month}.zst"))).await?);

  let mut next_comment: Option<Comment> = comment_stream.next().await.transpose()?;
  let mut next_post: Option<Submission> = post_stream.next().await.transpose()?;
  Ok(stream! {
    loop {
      match (&next_comment, &next_post) {
        (Some(c), p) if p.as_ref().map(|p| c.created_utc < p.created_utc).unwrap_or(true) => {
          yield Ok(SubmissionOrComment::Comment(next_comment.expect("must exist")));
          next_comment = comment_stream.next().await.transpose()?;
        }
        (Some(_), None) => panic!("impossible, covered above"),
        (_, Some(_)) => {
          yield Ok(SubmissionOrComment::Submission(next_post.expect("must exist")));
          next_post = post_stream.next().await.transpose()?;
        }
        (None, None) => { return }
      }
    }
  })
}
#[derive(Serialize, Debug)]
struct ToApub {
  #[serde(flatten)]
  raw: Value,
  id: Url,
  actor: Url,
}
#[async_trait]
impl ActivityHandler for ToApub {
  type DataType = ();

  type Error = anyhow::Error;

  fn id(&self) -> &Url {
    &self.id
  }

  fn actor(&self) -> &Url {
    &self.actor
  }

  async fn verify(&self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
    todo!()
  }

  async fn receive(self, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
    todo!()
  }
}

#[derive(Debug)]
struct RedditActor {}
#[async_trait]
impl Object for RedditActor {
  type DataType = ();

  type Kind = ();

  type Error = anyhow::Error;

  async fn read_from_id(
    _object_id: Url,
    _data: &Data<Self::DataType>,
  ) -> Result<Option<Self>, Self::Error> {
    todo!();
  }

  async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
    todo!();
  }

  async fn verify(
    _json: &Self::Kind,
    _expected_domain: &Url,
    _data: &Data<Self::DataType>,
  ) -> Result<(), Self::Error> {
    todo!()
  }
  async fn from_json(_json: Self::Kind, _data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
    todo!()
  }
}
impl Actor for RedditActor {
  fn id(&self) -> Url {
    Url::parse(SERVER_URL).unwrap()
  }
  fn public_key_pem(&self) -> &str {
    "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAuU/rSjt1HevYTcHaw4qI\nIzfpWEoJfE9j5iaftUrZKNW5D8Lh++IPbXKQmP8WxWL3jwWPYWWMkwzgYxTCXbl9\n7LredZ+uXdy4I4nr+HEp6nrt/5fU6qz2eOZlALLiaZnDXcB1UtbBNg5aH3uvM3Si\nl7AnwsbXvwx9yg32GoQ06xjMp152wcO3OcIghLbZM4pcGLhRltzuiu/h2u+t7pyi\nbmFW646veyHt+tuUZ6rxfoOyWx+rOekrAIHM6oMbp/D7QGovlejb68labStYckor\nXBcCk8ZlXX2hlRGvOe3RzMAqKDgqxDWZ/drxUEq7YkHU6Lw5L+cuuc+d5tjDdug2\nHwIDAQAB\n-----END PUBLIC KEY-----\n"
  }
  fn private_key_pem(&self) -> Option<String> {
    Some(String::from("-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC5T+tKO3Ud69hN\nwdrDiogjN+lYSgl8T2PmJp+1Stko1bkPwuH74g9tcpCY/xbFYvePBY9hZYyTDOBj\nFMJduX3sut51n65d3Lgjiev4cSnqeu3/l9TqrPZ45mUAsuJpmcNdwHVS1sE2Dlof\ne68zdKKXsCfCxte/DH3KDfYahDTrGMynXnbBw7c5wiCEttkzilwYuFGW3O6K7+Ha\n763unKJuYVbrjq97Ie3625RnqvF+g7JbH6s56SsAgczqgxun8PtAai+V6NvryVpt\nK1hySitcFwKTxmVdfaGVEa857dHMwCooOCrENZn92vFQSrtiQdTovDkv5y65z53m\n2MN26DYfAgMBAAECggEAHkbH7v9q4aIcW4vuJZ+XIYXrioDCLvy7mik6U8DwXQMa\nMtCI4oHrOlsK8+xNeJ90SfpDFEsmngnvCVEldnGteMWJPheCQhSjQy8wDg3TJtvB\n0c4pO9RZiqQ94VDYvB8is8kTgh7TP3U11Un8dIA8ZmMiA+k/65drX91LFcb+7F//\niMqGblqIV6PAzMp1Jr2fDspcFtVHOajt+bte5lMfndxeTswEI0kDBUQWTkvuzpYQ\nIcXTg2sZoV3btdK7Lxs/8Il69w4BxrTqhu/i+F0YOzpFbU9lW9leQ7EdjOlVuaXe\nx8ChvYU2qS5GG4Rrw6tnHImmlGdkQWXuS1NjJQ/LdQKBgQD6KVoGr3JyG0RtcO8Q\nhpWIufrSWn7xCOFqSjchswSucoY7gVS15tlmSa32Eun2CCxLftmLZ7LYJjpN472Z\n8CAwws/dgU7j0LLkhqSHLu99qEJeHHfCDKmR4Tggbkag6cpChXp7rlgtGH9ENQVc\nUTj9JfNWPz86t2KD22OVGIjp/QKBgQC9oxwjQatH3U1hxoOWGrzQG9KHA26qP40n\noqr6AP5LcP7uDn5tK2SsypovZgWiC0tYwiSZCADpM69vu46Ni25b0spmiIjIc3wX\nAuJXXyiBGv7ikP02roUvP5LnB3NwE7tIcC03llOHZqgd3Sf1CtB4zD8HBEL+7cC6\nnrGZOpIdSwKBgQC45Y1zuYN2US8PUNRxu3eUmhmIFnkSwESTog0DrGQ+Z8lM+/dX\nhyuSDc01Pp+MSFgs6LHz9o5ack7PuQ8vUysHv0WR63wap+tBOz8p54f9sTp0gsgF\nNgSzHOq2FavATWxAJJX2ClOD6UJPcHzo0eO0P7OOQKsEQ/zdhm8hCQRRJQKBgBNO\nV77/IIDgdtBNdXgCoNZO/s/f+ZQ7hBNU7DMnhrwHdOynbReQI1+0AJ5ytIAaxkDz\nAubRecZEDMhDP/AJEeMnQpPNsp81opx1HrXmaik6plhKinzWp5h30GzUxVvTpm1p\nfjD6jOZr/RGNQlQgFbk2kfQU6v0pF0XoggwnelihAoGBAOnc8Zs/tOu+Wz6CQk2z\nTo8Rce4Ur92cQVMT4xD2eVZS3owVGQw5JBxMeXl6XfmDPtJtOSzA4uePyhN6FW94\nLMaAZ/qOxQUbdh2/vadCjutAeQWL2IoGyOL/X31Ez6y9Fujt4F0jPgw2HAFD0pFK\no0TdM2sK20FaQDWzXZ3Pt4Ia\n-----END PRIVATE KEY-----\n"))
  }
  fn inbox(&self) -> Url {
    todo!();
  }
}

fn permalink(p: &str) -> Result<Url> {
  Ok(Url::parse(SERVER_URL)?.join(&p)?)
}
fn author_url(author: &str) -> Result<Url> {
  Ok(Url::parse(SERVER_URL)?.join("/u/")?.join(author)?)
}

fn context() -> Value {
  json!([
    "https://www.w3.org/ns/activitystreams",
    "https://w3id.org/security/v1",
    {
      "lemmy": "https://join-lemmy.org/ns#",
      "litepub": "http://litepub.social/ns#",
      "pt": "https://joinpeertube.org/ns#",
      "sc": "http://schema.org/",
      "ChatMessage": "litepub:ChatMessage",
      "commentsEnabled": "pt:commentsEnabled",
      "sensitive": "as:sensitive",
      "matrixUserId": "lemmy:matrixUserId",
      "postingRestrictedToMods": "lemmy:postingRestrictedToMods",
      "removeData": "lemmy:removeData",
      "stickied": "lemmy:stickied",
      "moderators": {
        "@type": "@id",
        "@id": "lemmy:moderators"
      },
      "expires": "as:endTime",
      "distinguished": "lemmy:distinguished",
      "language": "sc:inLanguage",
      "identifier": "sc:identifier"
    }
  ])
}

impl Submission {
  fn url(&self) -> Result<Url> {
    permalink(&self.permalink)
  }
  fn author_url(&self) -> Result<Url> {
    author_url(&self.author)
  }
  fn to_activitypub(&self) -> Result<Option<ToApub>> {
    let create_id = Url::parse(SERVER_URL)?
      .join("/activity-create/")?
      .join(&self.permalink[1..])?;
    let actor = self.author_url()?;
    let id = self.url()?;
    let community = permalink(&format!("/r/{}", self.subreddit))?;
    let body = if self.selftext.is_empty() {
      self.url.clone()
    } else {
      self.selftext.clone()
    };
    Ok(Some(ToApub {
      id: create_id.clone(),
      actor: actor.clone(),
      raw: json!({
      "@context": context(),
      "cc": [community],
      "audience": community,
      "type": "Create",
      "to": ["https://www.w3.org/ns/activitystreams#Public"],
      "object": {
        "type": "Page",
        "id": id,
        "attributedTo": actor,
        "to": [
          community,
          "https://www.w3.org/ns/activitystreams#Public"
        ],
        "audience": community,
        "name": self.title,
        "content": body,
        "mediaType": "text/html",
        "source": {
          "content": body,
          "mediaType": "text/markdown"
        },
        /*"attachment": [
          {
            "type": "Link",
            "href": "https://lemmy.ml/pictrs/image/xl8W7FZfk9.jpg"
          }
        ],*/
        "commentsEnabled": true,
        "sensitive": false,
        /*"language": {
          "identifier": "ko",
          "name": "한국어"
        },*/
        "published": time::OffsetDateTime::from_unix_timestamp(self.created_utc as i64)?.format(&Rfc3339)?
      },
      }),
    }))
  }
}

impl Comment {
  fn url(&self) -> Result<Url> {
    permalink(&self.permalink)
  }
  fn author_url(&self) -> Result<Url> {
    author_url(&self.author)
  }
  fn to_activitypub(
    &self,
    comments_cache: &HashMap<String, Comment>,
    posts_cache: &HashMap<String, Submission>,
  ) -> Result<Option<ToApub>> {
    let comment = self;
    let create_id = Url::parse(SERVER_URL)?
      .join("/activity-create/")?
      .join(&comment.permalink[1..])?;
    let comment_id = comment.url()?;
    let actor = comment.author_url()?;
    let community = permalink(&format!("/r/{}", self.subreddit))?; // intentionally mangle /u/ subreddits because that doesn't work in lemmy
    let time =
      time::OffsetDateTime::from_unix_timestamp(comment.created_utc as i64)?.format(&Rfc3339)?;
    let (parent_comment_author, parent_comment_url) = {
      if comment.parent_id.starts_with("t3_") {
        let post = posts_cache.get(&comment.parent_id[3..]);
        if let Some(post) = post {
          tracing::debug!("found parent post: {:?}", post);
          (post.author_url()?, post.url()?)
        } else {
          tracing::debug!(
            "skipping comment, parent post https://reddit.com/comments/{} not found",
            &comment.parent_id[3..]
          );
          return Ok(None);
        }
      } else if comment.parent_id.starts_with("t1_") {
        let parent_comment = comments_cache.get(&comment.parent_id[3..]);
        if let Some(parent_comment) = parent_comment {
          tracing::debug!("found parent comment: {:?}", comment);
          (parent_comment.author_url()?, parent_comment.url()?)
        } else {
          tracing::debug!(
            "skipping comment, parent comment https://reddit.com/comments/{} not found",
            &comment.parent_id[3..]
          );
          return Ok(None);
        }
      } else {
        anyhow::bail!("Cannot handle parent id {}", comment.parent_id);
      }
    };
    Ok(Some(ToApub {
      id: create_id.clone(),
      actor: actor.clone(),
      raw: json!({
        "@context": context(),
        "type": "Create",
        "to": ["https://www.w3.org/ns/activitystreams#Public"],
        "object": {
          "type": "Note",
          "id": comment_id,
          "attributedTo": actor.clone(),
          "to": ["https://www.w3.org/ns/activitystreams#Public"],
          "cc": [
            community,
            parent_comment_author
          ],
          "audience": community,
          "content": comment.body, // TODO: html
          "mediaType": "text/html",
          "source": {
            "content": comment.body,
            "mediaType": "text/markdown"
          },
          "inReplyTo": parent_comment_url,
          "published": time, // "2021-11-01T11:45:49.794920+00:00"
        },
        "cc": [
          community,
          parent_comment_author
        ],
        "audience": community,
        "tag": [
          {
            "type": "Mention",
            "href": parent_comment_author,
            "name": parent_comment_author // todo: should have format @cypherpunks@lemmy.ml apparently
          }
        ],
      }),
    }))
  }
}

/// Necessary because of this issue: https://github.com/actix/actix-web/issues/1711
#[derive(Debug)]
pub struct Error(pub(crate) anyhow::Error);

impl Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Display::fmt(&self.0, f)
  }
}

impl<T> From<T> for Error
where
  T: Into<anyhow::Error>,
{
  fn from(t: T) -> Self {
    Error(t.into())
  }
}
impl ResponseError for Error {}

#[get("/r/{name}")]
async fn http_get_community(name: web::Path<String>) -> std::result::Result<impl Responder, Error> {
  let url = permalink(&format!("/r/{name}"))?;
  Ok(
      HttpResponse::Ok()
        .content_type(ContentType(
          mime::Mime::from_str("application/activity+json").unwrap(),
        ))
        .json(json!({
          "@context": context(),
          "type": "Group",
          "id": url,
          "preferredUsername": name.to_string(),
          "inbox": permalink(&format!("/r/{name}/inbox"))?,
          "followers":permalink(&format!("/r/{name}/followers"))?,
          "publicKey": { // don't care
            "id": "https://lemmy.world/c/syncforlemmy#main-key",
            "owner": "https://lemmy.world/c/syncforlemmy",
            "publicKeyPem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEApjSiWhzx6mY7s4qjfjU7\nRq72lFZ0Mcci4B9pv152/Ikihwt99lmwgy2bb1lPTBWhh4RLsa6qT8ilhgVMBE8f\n74pKo7MT+ZXmsQz4miFG9NoDIMFPhoUadUnISVxLtHWYx1bLvt8mpFinKJFT297T\npFgueBAcrgnu407mC/6XCaf/pKWwYzGOMqrOLxuviQ4s+vrPn73kaIRi177YRJ9Q\nnIjV5r2pFQavH0flQdQzLa/1O5paBOJPise8GzItiia6RJ2MSSN9R1R0efefN90E\ntkAJpVEOwaICST6gkMCImY09nqHrLkBlmEciGShBvqpk1vVIoXhY9P8qi0CYVzoB\n5wIDAQAB\n-----END PUBLIC KEY-----\n"
          },
          "name": name.to_string(),
          "summary": "loaded from redidt dump",
          "sensitive": false,
          // "moderators": "https://lemmy.world/c/syncforlemmy/moderators",
          // "attributedTo": "https://lemmy.world/c/syncforlemmy/moderators",
          "postingRestrictedToMods": false,
          "outbox": permalink(&format!("/r/{name}/outbox"))?,
          /*"endpoints": {
            "sharedInbox": "https://lemmy.world/inbox"
          },*/
        })),
    )
}
#[get("/u/{name}")]
async fn http_get_user(name: web::Path<String>) -> std::result::Result<impl Responder, Error> {
  Ok(
    HttpResponse::Ok()
      .content_type(ContentType(
        mime::Mime::from_str("application/activity+json").unwrap(),
      ))
      .json(json!({
        "id": Url::parse(SERVER_URL)?.join(&format!("/u/{name}"))?,
        "type": "Person",
        "preferredUsername": name.to_string(),
        "name": name.to_string(),
        // "summary": "<p>Captain of the starship <strong>Enterprise</strong>.</p>\n",
        /*"source": {
          "content": "Captain of the starship **Enterprise**.",
          "mediaType": "text/markdown"
        },*/
        /*"icon": {
          "type": "Image",
          "url": "https://enterprise.lemmy.ml/pictrs/image/ed9ej7.jpg"
        },
        "image": {
          "type": "Image",
          "url": "https://enterprise.lemmy.ml/pictrs/image/XenaYI5hTn.png"
        },*/
        // "matrixUserId": "@picard:matrix.org",
        "inbox": Url::parse(SERVER_URL)?.join(&format!("/u/{name}/inbox"))?,
        "outbox": Url::parse(SERVER_URL)?.join(&format!("/u/{name}/outbox"))?,
        /* "endpoints": {
          "sharedInbox": "https://enterprise.lemmy.ml/inbox"
        },*/
        // "published": "2020-01-17T01:38:22.348392+00:00",
        // "updated": "2021-08-13T00:11:15.941990+00:00",
        "publicKey": {
          "id": SERVER_URL,
          "owner": SERVER_URL,
          "publicKeyPem": RedditActor {}.public_key_pem()
        }
      })),
  )
}

pub async fn go() -> Result<()> {
  tracing_subscriber::fmt::init();
  let opt = Options::parse();
  let mut stream = pin!(
    import_merged_reddit_dump(&Path::new(&opt.input_dir), "2022-12")
      .await?
      .skip((opt.skip - 1).try_into()?)
  );
  {
    let now = Instant::now();
    stream.next().await; // ensure one is read to enforce skipping now
    tracing::info!("skipping {} took {:.2?}", opt.skip, now.elapsed());
  }
  let reddit_actor = RedditActor {};
  let fed = FederationConfig::builder()
    .domain("reddit.com.local")
    .debug(false)
    .allow_http_urls(true)
    .app_data(())
    .worker_count(100)
    .build()
    .await?;
  let server = HttpServer::new(|| {
    App::new()
      .service(http_get_user)
      .service(http_get_community)
  })
  .bind((
    "127.0.0.1",
    opt.local_server.port_or_known_default().unwrap(),
  ))?
  .disable_signals()
  .run();
  // server.await;

  tokio::task::spawn(server);
  let data = fed.to_request_data();
  // keep seen posts/comments in memory to make child comments reference them
  let mut posts_cache: HashMap<String, Submission> = HashMap::new();
  let mut comments_cache: HashMap<String, Comment> = HashMap::new();
  let mut count = 0;

  let start = Instant::now();
  while let Some(res) = stream.next().await {
    if count >= opt.limit {
      tracing::info!("reached limit of {count} sends, exiting");
      break;
    }
    let inboxes = vec![opt.remote_server.clone()];
    match res? {
      SubmissionOrComment::Submission(post) => {
        let post_apub = post.to_activitypub().context("converting post to apub")?;
        if let Some(post_apub) = post_apub {
          tracing::info!(
            "sending post {} {}",
            post.id,
            post_apub.raw["object"]["published"]
          );
          posts_cache.insert(post.id.clone(), post.clone());
          count += 1;
          send_activity(post_apub, &reddit_actor, inboxes.clone(), &data).await?;
        }
      }
      SubmissionOrComment::Comment(comment) => {
        let comment_apub = comment
          .to_activitypub(&comments_cache, &posts_cache)
          .context("converting comment to apub")?;
        if let Some(comment_apub) = comment_apub {
          tracing::info!(
            "sending comment {} {}",
            comment.id,
            comment_apub.raw["object"]["published"]
          );
          comments_cache.insert(comment.id.clone(), comment.clone());
          count += 1;
          send_activity(comment_apub, &reddit_actor, inboxes.clone(), &data).await?;
        }
      }
    }
  }
  tracing::warn!("sending {} took {:.2?}", count, start.elapsed());
  drop(data);
  let start = Instant::now();
  fed.shutdown(false).await?;
  tracing::warn!("clearing queue took {:.2?}", start.elapsed());
  Ok(())
}
