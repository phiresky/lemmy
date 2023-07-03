use activitypub_federation::traits::{ActivityHandler, Actor};
use anyhow::{Context, Result};
use async_stream::stream;
use clap::Parser;
use diesel::QueryableByName;
use futures::StreamExt;
use futures_core::stream::Stream;
use indexmap::IndexSet;
use integration_testing::{import_zstd_json_dump, jsonld_context, ToApub};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
  cmp::{Ordering, Reverse},
  collections::{BinaryHeap, HashMap},
  fs::File,
  io::Write,
  path::Path,
  pin::pin,
  task::Poll,
  time::{Duration, Instant},
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::mpsc::UnboundedReceiver;
use url::Url;
const SERVER_URL: &str = "http://reddit.com.localhost:5313/"; // TODO: read from options

#[derive(Debug, Parser, Serialize)]
struct ConvertOptions {
  /// the root directory of the reddit dump. this directory should contain the files `comments/RC_2022-12.zst` and `submissions/RS_2022-12.zst`
  #[arg(long)]
  input_dir: String,
  /// when this number of events have been sent, stop
  #[arg(default_value_t = 100000, long)]
  limit: i64,
  /// skip this number of entries from the input files to make the ratio from comment to post more realistic (because many comments are for older posts we don't have)
  #[arg(default_value_t = 0, long)]
  skip: i64,
  /// how many comments a post should get on average. If none, don't adjust ratio. this parameter is needed for realism
  /// because many comments are for older posts that don't exist in the stream
  /// 2022-12 has 238862690 / 35865148 = 6.66
  /// post to score ratio: 4463366 / 100000 = 45
  /// comment to score ratio: 708176 / 100000 = 7
  #[arg(long, default_value = "6.66")]
  comment_to_post_ratio: Option<f64>,
  /// output file (format jsonl)
  #[arg(long)]
  output_file: String,
}

//
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
  score: i64,
  num_comments: i64,
  upvote_ratio: f64,
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
  score: i64,
  // seconds. 1669852800
  created_utc: f64,
  controversiality: i64,
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

#[derive(Clone, Serialize)]
enum LikeOrD {
  Like,
  Dislike,
}
#[derive(Clone)]
enum SubmissionOrComment {
  Submission(Submission),
  Comment(Comment),
  Vote(Vote),
}
#[derive(Clone)]
struct Vote {
  created_utc: OffsetDateTime,
  id: Url,
  actor: Url,
  object: Url,
  r#type: LikeOrD,
  audience: Url,
}
impl Vote {
  fn to_activitypub(&self) -> Result<ToApub> {
    Ok(ToApub {
      id: self.id.clone(),
      actor: self.actor.clone(),
      raw: json!({
        "object": self.object,
        "type": self.r#type,
        "audience": self.audience,
        "created": self.created_utc.format(&Rfc3339)?
      }),
    })
  }
}
impl SubmissionOrComment {
  fn created(&self, delay_comments_seconds: f64) -> OffsetDateTime {
    // delay all comments by this input time to make race condition less likely
    match self {
      SubmissionOrComment::Submission(s) => s.created(),
      SubmissionOrComment::Comment(s) => {
        s.created() + Duration::from_secs_f64(delay_comments_seconds)
      }
      SubmissionOrComment::Vote(v) => v.created_utc,
    }
  }
}
impl PartialEq for SubmissionOrComment {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Submission(l0), Self::Submission(r0)) => l0.id == r0.id,
      (Self::Comment(l0), Self::Comment(r0)) => l0.id == r0.id,
      _ => false,
    }
  }
}
impl Eq for SubmissionOrComment {}
impl Ord for SubmissionOrComment {
  fn cmp(&self, other: &Self) -> Ordering {
    self.partial_cmp(other).unwrap()
  }
}
impl PartialOrd for SubmissionOrComment {
  fn partial_cmp(&self, other: &Self) -> std::option::Option<std::cmp::Ordering> {
    self.created(10.0).partial_cmp(&other.created(10.0))
  }
}

fn merge_streams<T>(
  a: impl Stream<Item = T>,
  b: impl Stream<Item = T>,
  less_than: impl Fn(&T, &T) -> bool,
) -> impl Stream<Item = T> {
  // let mut vote_stream = BinaryHeap::new();
  let mut a = Box::pin(a);
  let mut b = Box::pin(b);
  stream! {
    let mut next_a: Option<T> = a
      .next()
      .await;
    let mut next_b: Option<T> = b
      .next()
      .await;
    loop {
      match (&next_a, &next_b) {
        (Some(c), p) if p.as_ref().map(|p| less_than(c, p)).unwrap_or(true) => {
          yield next_a.expect("must exist");
          next_a = a.next().await;
        }
        (Some(_), None) => panic!("impossible, covered above"),
        (_, Some(_)) => {
          yield next_b.expect("must exist");
          next_b = b.next().await;
        }
        (None, None) => { return }
      }
    }
  }
}
/// load the comments and posts file and intermingle them ordered by time
async fn import_merged_reddit_dump(
  source_dir: &Path,
  month: &str,
) -> Result<impl Stream<Item = Result<SubmissionOrComment>>> {
  let comment_stream = Box::pin(
    import_zstd_json_dump(&source_dir.join(format!("comments/RC_{month}.zst")))
      .await
      .context("opening comments")?
      .map(|s| s.map(SubmissionOrComment::Comment)),
  );
  let post_stream = Box::pin(
    import_zstd_json_dump(&source_dir.join(format!("submissions/RS_{month}.zst")))
      .await
      .context("opening submissions")?
      .map(|s| s.map(SubmissionOrComment::Submission)),
  );
  Ok(merge_streams(comment_stream, post_stream, less_than_r))
}

fn less_than_r(a: &Result<SubmissionOrComment>, b: &Result<SubmissionOrComment>) -> bool {
  match (a, b) {
    (Ok(a), Ok(b)) => a.lt(b),
    (Ok(_), Err(_)) => false,
    (Err(_), _) => true,
  }
}

#[pin_project::pin_project]
struct FakeVotes {
  votes: BinaryHeap<Reverse<SubmissionOrComment>>,
  users: IndexSet<String>,
  receiver: UnboundedReceiver<SubmissionOrComment>,
}
/* impl FakeVotes {
  async fn add(&self, r: &Result<SubmissionOrComment>) {
    let Ok(r) = r else { return };
    let votes = self.votes.lock().await;
  }
}*/
impl Stream for FakeVotes {
  type Item = Result<SubmissionOrComment>;

  // fn add_vote()
  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let done = loop {
      match this.receiver.poll_recv(cx) {
        Poll::Ready(Some(r)) => {
          let (id, community, time, (upvotes, downvotes)) = match r {
            SubmissionOrComment::Vote { .. } => panic!("impossibl"),
            SubmissionOrComment::Submission(s) => {
              this.users.insert(s.author.clone());
              (
                s.url().unwrap(),
                s.community_url().unwrap(),
                s.created(),
                compute_votes(s.score, s.upvote_ratio),
              )
            }
            SubmissionOrComment::Comment(c) => {
              this.users.insert(c.author.clone());
              (
                c.url().unwrap(),
                c.community_url().unwrap(),
                c.created(),
                if c.score < 0 {
                  compute_votes(c.score, 0.2)
                } else if c.controversiality != 0 {
                  compute_votes(c.score, 0.6)
                } else {
                  compute_votes(c.score, 0.9)
                },
              )
            }
          };
          if upvotes + downvotes > 1000 {
            tracing::warn!("{}>1000 votes on {}!", upvotes + downvotes, id);
          }
          let mut rng = rand::thread_rng();
          for i in 0..upvotes + downvotes {
            let r#type = if i >= upvotes {
              LikeOrD::Dislike
            } else {
              LikeOrD::Like
            };
            let user = this
              .users
              .get_index(rng.gen_range(0..this.users.len()))
              .unwrap();
            let id = id.clone();
            this.votes.push(Reverse(SubmissionOrComment::Vote(Vote {
              id: permalink(&format!("activity-vote/{id}.{i}")).unwrap(),
              created_utc: time + Duration::from_secs(rng.gen_range(0..2 * 60 * 60)),
              actor: author_url(user).unwrap(),
              object: id,
              r#type,
              audience: community.clone(),
            })));
          }
        }
        Poll::Ready(None) => break true,

        Poll::Pending => break false,
      }
    };
    if let Some(item) = this.votes.pop() {
      return Poll::Ready(Some(Ok(item.0)));
    }
    if done {
      Poll::Ready(None)
    } else {
      Poll::Pending
    }
  }
}
fn add_fake_votes(
  stream: impl Stream<Item = Result<SubmissionOrComment>>,
) -> impl Stream<Item = Result<SubmissionOrComment>> {
  let (s, receiver) = tokio::sync::mpsc::unbounded_channel();
  let votes = FakeVotes {
    votes: BinaryHeap::new(),
    receiver,
    users: IndexSet::new(),
  };
  let stream = {
    //let votes = votes.clone();
    stream
      .inspect(move |item| {
        let Ok(r) = item else { return; };
        let item = (*r).clone();
        s.send(item).map_err(|_| "cuodnlet send").unwrap();
      })
      .skip(10) // preload a few
  };
  merge_streams(stream, votes, less_than_r)
}

fn permalink(p: &str) -> Result<Url> {
  Ok(Url::parse(SERVER_URL)?.join(p)?)
}
fn author_url(author: &str) -> Result<Url> {
  Ok(Url::parse(SERVER_URL)?.join("/u/")?.join(author)?)
}

impl Submission {
  fn url(&self) -> Result<Url> {
    permalink(&self.permalink)
  }
  fn author_url(&self) -> Result<Url> {
    author_url(&self.author)
  }
  fn community_url(&self) -> Result<Url> {
    permalink(&format!("/r/{}", self.subreddit))
  }
  fn created(&self) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(self.created_utc as i64).unwrap()
  }
  fn to_activitypub(&self) -> Result<Option<ToApub>> {
    let create_id = Url::parse(SERVER_URL)?
      .join("/activity-create/")?
      .join(&self.permalink[1..])?;
    let actor = self.author_url()?;
    let id = self.url()?;
    let community = self.community_url()?;
    let body = if self.selftext.is_empty() {
      self.url.clone()
    } else {
      self.selftext.clone()
    };
    Ok(Some(ToApub {
      id: create_id,
      actor: actor.clone(),
      raw: json!({
      "@context": jsonld_context(),
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
        "published": self.created().format(&Rfc3339)?
      },
      }),
    }))
  }
}

impl Comment {
  fn url(&self) -> Result<Url> {
    permalink(&self.permalink)
  }
  fn community_url(&self) -> Result<Url> {
    // intentionally mangle /u/ subreddits because that doesn't work in lemmy (having a url be both group and user)
    permalink(&format!("/r/{}", self.subreddit))
  }
  fn author_url(&self) -> Result<Url> {
    author_url(&self.author)
  }
  fn created(&self) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(self.created_utc as i64).unwrap()
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
    let community = self.community_url()?;
    let time = self.created().format(&Rfc3339)?;
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
      id: create_id,
      actor: actor.clone(),
      raw: json!({
        "@context": jsonld_context(),
        "type": "Create",
        "to": ["https://www.w3.org/ns/activitystreams#Public"],
        "object": {
          "type": "Note",
          "id": comment_id,
          "attributedTo": actor,
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

fn compute_votes(score: i64, ratio: f64) -> (i64, i64) {
  if ratio < 0.51 && score > 0 {
    // outlier case, total is probably wrong maybe
    return (score, 0);
  }
  let upvotes = ((score as f64) / (2.0 - 1.0 / ratio)).round() as i64;
  let downvotes = upvotes - score;
  (upvotes, downvotes)
}

#[tokio::main]
pub async fn main() -> Result<()> {
  tracing_subscriber::fmt::init();
  let opt = ConvertOptions::parse();
  let mut outfile = zstd::stream::Encoder::new(
    File::create(opt.output_file).context("creating output file")?,
    19,
  )?.auto_finish();

  let mut stream = pin!(add_fake_votes(
    import_merged_reddit_dump(Path::new(&opt.input_dir), "2022-12")
      .await?
      .skip(if opt.skip > 0 {
        (opt.skip - 1) as usize
      } else {
        0
      })
  ));
  if opt.skip > 0 {
    let now = Instant::now();
    stream.next().await; // ensure one is read to enforce skipping now
    tracing::info!("skipping {} took {:.2?}", opt.skip, now.elapsed());
  }

  // keep seen posts/comments in memory to make child comments reference them
  let mut posts_cache: HashMap<String, Submission> = HashMap::new();
  let mut comments_cache: HashMap<String, Comment> = HashMap::new();
  let mut total_send_count = 0;
  let mut comment_send_count = 0;
  let mut post_send_count = 0;
  let mut post_skip_count = 0;
  let mut iteration_count = 0;
  let mut vote_send_count = 0;

  let start = Instant::now();
  while let Some(res) = stream.next().await {
    let res = res?;
    iteration_count += 1;
    if total_send_count >= opt.limit {
      tracing::info!("reached limit of {total_send_count} sends, exiting");
      break;
    }
    let comment_to_post_ratio = (comment_send_count as f64) / (post_send_count as f64);
    let cur_time = res.created(0.0);
    if iteration_count % 10000 == 0 {
      let limit = opt.limit;
      let now = cur_time.format(&Rfc3339)?;
      tracing::warn!(
        "sent {total_send_count} (posts={post_send_count}, comments={comment_send_count}, votes={vote_send_count}, post_skip={post_skip_count}) of {limit}, saw {iteration_count}. c/p:{comment_to_post_ratio:.2} (cur time: {now})",
      );
    }
    match res {
      SubmissionOrComment::Submission(post) => {
        if opt
          .comment_to_post_ratio
          .map(|ratio| post.num_comments < 10 && comment_to_post_ratio / ratio < 0.95)
          .unwrap_or(false)
        {
          post_skip_count += 1;
          continue;
        }
        let post_apub = post.to_activitypub().context("converting post to apub")?;
        if let Some(post_apub) = post_apub {
          posts_cache.insert(post.id.clone(), post.clone());
          total_send_count += 1;
          post_send_count += 1;
          serde_json::to_writer(&mut outfile, &post_apub)?; // todo: this is blocking
          outfile.write(b"\n")?;
        }
      }
      SubmissionOrComment::Comment(comment) => {
        let comment_apub = comment
          .to_activitypub(&comments_cache, &posts_cache)
          .context("converting comment to apub")?;
        if let Some(comment_apub) = comment_apub {
          comments_cache.insert(comment.id.clone(), comment.clone());
          total_send_count += 1;
          comment_send_count += 1;
          serde_json::to_writer(&mut outfile, &comment_apub)?; // todo: this is blocking
          outfile.write(b"\n")?;
        }
      }
      SubmissionOrComment::Vote(vote) => {
        let vote_apub = vote
          .to_activitypub()
          .context("converting comment to apub")?;
        total_send_count += 1;
        vote_send_count += 1;
        serde_json::to_writer(&mut outfile, &vote_apub)?; // todo: this is blocking
        outfile.write(b"\n")?;
      }
    }
  }
  let time = start.elapsed();
  tracing::warn!("converting {total_send_count} took {time:.2?}");
  Ok(())
}
