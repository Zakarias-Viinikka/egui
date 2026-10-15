~/ProgStuff > head -40 /home/zakke/ProgStuff/ZakkeN/STORED_AI_CONTEXT.md
# ZakkeN — AI stored context

Written by AI for AI. Meant to be the first file a fresh AI reads when
starting work on this project. Update a section when its content
changes and bump that section's date. If code and this file disagree,
the code wins and this file should be corrected.

## What this is — 2026-10-13

ZakkeN is an offline-first Notion-style notes app for personal use, on
Android and the web. The goal is to store, edit, and find old notes
easily, sync seamlessly, need almost no config, and have a better UX
than what exists.

The domain is pages and blocks. A page is a list of blocks. A block is
text plus metadata. Every page's state lives in a yrs CRDT doc; the
sqlite mirror is queryable but not authoritative. Edits are described
as "love letters" — a serialized intent the client sends to the server
so both sides agree on what changed. The client stores its own copy in
uncommitted_diffs while the server tracks which registered clients have
received which letters.

The server now works end to end for the shapes that exist. See the
zakken_server section below.

## Crate map — 2026-10-14

yrs (ZakkeN/yrs) — CRDT wrappers. BossOfYrs owns one page's doc,
YrsActivePages owns the "which pages are disabled" doc, YrsBacklinks
owns backlinks. Wasm has no threads and no clock, so BossOfYrs::new
takes a unix_time string; IDs are built from that string, an atomic
counter, and a random part. anti_deadlock.rs wraps every public method
with a timeout; DEBUG_MODE is currently false on all targets (wasm
would panic on thread::spawn), which is a stopgap. The crate's lib name
is my_yrs_lib even though the package name is yrs — imports say
my_yrs_lib, Cargo.toml lines say yrs.path.

client_table_blueprints (ZakkeN/client_table_blueprints) — the schema.
tbl_pages, tbl_every_block_in_existence, tbl_uncommitted_diffs,
tbl_incoming_love_letters, tbl_backlinks, tbl_key_value_storage,
copied 41 lines
~/ProgStuff > cat /home/zakke/ProgStuff/ZakkeN/zakken_server/Cargo.toml /home/zakke/ProgStuff/ZakkeN/zakken_server/db/Cargo.toml /home/zakke/ProgStuff/ZakkeN/zakken_server/error_stuff/Cargo.toml /home/zakke/ProgStuff/ZakkeN/zakken_server/love_letter_parser/Cargo.toml /home/zakke/ProgStuff/ZakkeN/zakken_server/server_core/Cargo.toml /home/zakke/ProgStuff/ZakkeN/zakken_server/server_protocol/Cargo.toml /home/zakke/ProgStuff/ZakkeN/zakken_server/mock_client/Cargo.toml /home/zakke/ProgStuff/ZakkeN/zakken_server/mock_client/mock_client_core/Cargo.toml /home/zakke/ProgStuff/ZakkeN/zakken_server/mock_client/cli_gui/Cargo.toml /home/zakke/ProgStuff/ZakkeN/zakken_server/local_cargo.sh /home/zakke/ProgStuff/ZakkeN/zakken_server/mock_client/local_cargo.sh
cat: couldn't read /home/zakke/ProgStuff/ZakkeN/zakken_server/mock_client/Cargo.toml
copied /home/zakke/ProgStuff/ZakkeN/zakken_server/Cargo.toml, /home/zakke/ProgStuff/ZakkeN/zakken_server/db/Cargo.toml, /home/zakke/ProgStuff/ZakkeN/zakken_server/error_stuff/Cargo.toml, /home/zakke/ProgStuff/ZakkeN/zakken_server/love_letter_parser/Cargo.toml, /home/zakke/ProgStuff/ZakkeN/zakken_server/server_core/Cargo.toml, /home/zakke/ProgStuff/ZakkeN/zakken_server/server_protocol/Cargo.toml, /home/zakke/ProgStuff/ZakkeN/zakken_server/mock_client/mock_client_core/Cargo.toml, /home/zakke/ProgStuff/ZakkeN/zakken_server/mock_client/cli_gui/Cargo.toml, /home/zakke/ProgStuff/ZakkeN/zakken_server/local_cargo.sh, /home/zakke/ProgStuff/ZakkeN/zakken_server/mock_client/local_cargo.sh to clipboard (failed: /home/zakke/ProgStuff/ZakkeN/zakken_server/mock_client/Cargo.toml)
~/ProgStuff > overwrite /home/zakke/ProgStuff/ZakkeN/zakken_server/Cargo.toml
@new
[workspace]
members = [
    "server_side/db",
    "server_side/server_core",
    "shared/error_stuff",
    "server_side/love_letter_parser",
    "shared/server_protocol",
    "mock_client/mock_client_core",
    "mock_client/cli_gui",
]
resolver = "3"
@end

replace /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/Cargo.toml
@old
db_core = { path = "../db" }
server_protocol = { path = "../server_protocol" }
error_stuff = { path = "../error_stuff" }
love_letter_parser = { path = "../love_letter_parser" }
@new
db_core = { path = "../db" }
server_protocol = { path = "../../shared/server_protocol" }
error_stuff = { path = "../../shared/error_stuff" }
love_letter_parser = { path = "../love_letter_parser" }
@end

replace /home/zakke/ProgStuff/ZakkeN/zakken_server/mock_client/mock_client_core/Cargo.toml
@old
server_protocol = { path = "../../server_protocol" }
error_stuff = { path = "../../error_stuff" }
@new
server_protocol = { path = "../../shared/server_protocol" }
error_stuff = { path = "../../shared/error_stuff" }
@end
~/ProgStuff > cat /home/zakke/ProgStuff/rust_practice/server_template_3/server_side/queue/Cargo.toml /home/zakke/ProgStuff/rust_practice/server_template_3/server_side/queue/src/lib.rs /home/zakke/ProgStuff/rust_practice/server_template_3/server_side/queue/src/job.rs /home/zakke/ProgStuff/rust_practice/server_template_3/server_side/queue/src/queue_manager.rs /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/request.rs /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/src/allowed_requests/mod.rs
copied /home/zakke/ProgStuff/rust_practice/server_template_3/server_side/queue/Cargo.toml, /home/zakke/ProgStuff/rust_practice/server_template_3/server_side/queue/src/lib.rs, /home/zakke/ProgStuff/rust_practice/server_template_3/server_side/queue/src/job.rs, /home/zakke/ProgStuff/rust_practice/server_template_3/server_side/queue/src/queue_manager.rs, /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/request.rs, /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/src/allowed_requests/mod.rs to clipboard
~/ProgStuff > create /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/queue/Cargo.toml
@new
[package]
name = "queue"
version = "0.1.0"
edition = "2024"

[dependencies]
server_protocol = { path = "../../shared/server_protocol" }
tokio = { version = "1.53.1", features = ["sync", "rt", "macros"] }
@end

create /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/queue/src/lib.rs
@new
mod job;
mod queue_manager;

pub use job::{Job, JobReply, Priority};
pub use queue_manager::QueueManager;
@end

create /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/queue/src/job.rs
@new
use server_protocol::Request;
use tokio::sync::oneshot;

pub enum Priority {
    Important,
    Normal,
}

pub struct Job {
    pub message_id: usize,
    pub request: Request,
    pub payload: Vec<u8>,
    pub sender: Option<String>,
    pub reply: oneshot::Sender<JobReply>,
}

pub struct JobReply {
    pub bytes: Vec<u8>,
    pub just_identified: Option<String>,
}
@end

create /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/queue/src/queue_manager.rs
@new
use crate::job::{Job, JobReply, Priority};
use server_protocol::Request;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct QueueManager {
    important_tx: mpsc::UnboundedSender<Job>,
    normal_tx: mpsc::UnboundedSender<Job>,
}

impl QueueManager {
    pub fn new(
        important_tx: mpsc::UnboundedSender<Job>,
        normal_tx: mpsc::UnboundedSender<Job>,
    ) -> Self {
        Self {
            important_tx,
            normal_tx,
        }
    }

    pub fn submit(
        &self,
        message_id: usize,
        request: Request,
        payload: Vec<u8>,
        sender: Option<String>,
        priority: Priority,
    ) -> Result<oneshot::Receiver<JobReply>, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let job = Job {
            message_id,
            request,
            payload,
            sender,
            reply: reply_tx,
        };

        let tx = match priority {
            Priority::Important => &self.important_tx,
            Priority::Normal => &self.normal_tx,
        };
        tx.send(job).map_err(|e| format!("queue is closed: {}", e))?;

        Ok(reply_rx)
    }
}
@end

replace /home/zakke/ProgStuff/ZakkeN/zakken_server/Cargo.toml
@old
    "server_side/love_letter_parser",
    "shared/server_protocol",
@new
    "server_side/love_letter_parser",
    "server_side/queue",
    "shared/server_protocol",
@end
~/ProgStuff > cat /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/src/connected_clients.rs /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/src/server_state.rs /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/src/lib.rs
copied /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/src/connected_clients.rs, /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/src/server_state.rs, /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/src/lib.rs to clipboard
~/ProgStuff > cat /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/error_stuff/src/lib.rs
copied /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/error_stuff/src/lib.rs to clipboard
~/ProgStuff > cat /home/zakke/ProgStuff/ZakkeN/love_letter/src/lib.rs /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/db/src/db/operations_love_letters.rs /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/src/main.rs
copied /home/zakke/ProgStuff/ZakkeN/love_letter/src/lib.rs, /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/db/src/db/operations_love_letters.rs, /home/zakke/ProgStuff/ZakkeN/zakken_server/server_side/server_core/src/main.rs to clipboard
~/ProgStuff > replace /home/zakke/ProgStuff/ZakkeN/love_letter/src/lib.rs
@old
#[uniffi::export]
pub fn build_a_love_letter(love_letter: LoveLetterSketch, snapshot_of_edit: Vec<u8>) -> LoveLetter {
@new
#[derive(uniffi::Record, Serialize, Deserialize)]
pub struct LoveLetterResponse {
    pub love_letter: LoveLetter,
    /// this is the id the client uses to keep track of whether it's received a love letter twice.
    /// this can happen if it receives a love letter, the connection dies, and then it reconnects,
    /// the server would send the love letter again since it never received an acknowledgment since server crashed.
    /// alternative would be to wrap the entire thing in a transaction client side, but that means
    /// every love letter starts a transaction and blocks other transactions, which is a problem
    /// since every edit to a page is a transaction.
    pub letter_id: i64,
}

#[uniffi::export]
pub fn build_a_love_letter(love_letter: LoveLetterSketch, snapshot_of_edit: Vec<u8>) -> LoveLetter {
@end
~/ProgStuff > replace /home/zakke/ProgStuff/ZakkeN/love_letter/src/lib.rs
@old
#[uniffi::export]
pub fn deserialize_love_letter(bytes: Vec<u8>) -> Result<LoveLetter, BrokenHeart> {
    bincode::deserialize(&bytes).map_err(Into::into)
}
@new
#[uniffi::export]
pub fn deserialize_love_letter(bytes: Vec<u8>) -> Result<LoveLetter, BrokenHeart> {
    bincode::deserialize(&bytes).map_err(Into::into)
}

#[uniffi::export]
pub fn love_letter_response_to_bytes(
    response: &LoveLetterResponse,
) -> Result<Vec<u8>, BrokenHeart> {
    bincode::serialize(response).map_err(Into::into)
}

#[uniffi::export]
pub fn deserialize_love_letter_response(
    bytes: Vec<u8>,
) -> Result<LoveLetterResponse, BrokenHeart> {
    bincode::deserialize(&bytes).map_err(Into::into)
}
@end
~/ProgStuff > cat /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/message.rs /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/builders.rs /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/lib.rs
copied /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/message.rs, /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/builders.rs, /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/lib.rs to clipboard
~/ProgStuff > replace /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/request.rs
@old
pub enum Request {
    Register,
    Login,
    SendLoveLetter,
    GetAllYrsPages,
}
@new
pub enum Request {
    Register,
    Login,
    SendLoveLetter,
    GetAllYrsPages,
    /// The client telling the server it has applied a love letter, so the
    /// server can delete that user's unsent row. Duplicates are harmless:
    /// the client dedups by letter_id, and a repeated ack for an already
    /// deleted row is a no-op.
    Ack,
}
@end

replace /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/request.rs
@old
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GetAllYrsPagesPayload {
    pub pages: Vec<PageRow>,
}
@new
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GetAllYrsPagesPayload {
    pub pages: Vec<PageRow>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AckPayload {
    pub letter_id: i64,
}
@end

replace /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/lib.rs
@old
pub use request::{
    GetAllYrsPagesPayload, LoginPayload, PageRow, RegisterPayload, Request, SendLoveLetterPayload,
};
@new
pub use request::{
    AckPayload, GetAllYrsPagesPayload, LoginPayload, PageRow, RegisterPayload, Request,
    SendLoveLetterPayload,
};
@end
~/ProgStuff > retry 1
replace /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/request.rs
@old
pub enum Request {
    Register,
    Login,
    SendLoveLetter,
    GetAllYrsPages,
}
@new
pub enum Request {
    Register,
    Login,
    SendLoveLetter,
    GetAllYrsPages,
    /// how a client acknowledges it's received a love letter response
    Ack,
}
@end
zapgui: step 1 of 1 (replace /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/request.rs) failed: expected 1 occurrence(s) of the old text in /home/zakke/ProgStuff/ZakkeN/zakken_server/shared/server_protocol/src/request.rs, found 0

## this didnt copy to clipboard on fail

## it also applied even tho i marked it for retry and after i clicked confirm, it applied it anyway. how does it work? does it expect me to deny after marking and then it applies everything once i confirm all again?