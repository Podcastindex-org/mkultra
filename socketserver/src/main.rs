use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
    fmt
};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
use std::thread;
use std::time;
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use serde_json::json;
use secp256k1::{schnorr::Signature, XOnlyPublicKey, SECP256K1};
use sha256;
use thiserror::Error;
use mkultra;

//TODO: Comment this code!
pub struct Chat {
    pub id: String
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let args: Vec<String> = env::args().collect();
    let arg_port = &args[1];

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let bind_addr = format!("0.0.0.0:{}", arg_port);
    let addr = bind_addr.clone();
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");


    //Make a keep-alive thread
    let peer_map = state.clone();
    thread::spawn(move || {
        loop {
            println!("Sending keep-alives...");

            // We want to broadcast the message to everyone except ourselves.
            let peers = peer_map.lock().unwrap();
            let broadcast_recipients =
                peers.iter().filter(|(peer_addr, _)| peer_addr == peer_addr).map(|(_, ws_sink)| ws_sink);

            for recp in broadcast_recipients {
                println!("  ...to Peer: {:#?}", recp);
                let result = recp.unbounded_send(Message::Text("{}".to_string()));
                match result {
                    Ok(_) => {
                        println!("  Ping sent.");
                    }
                    Err(e) => {
                        eprintln!("  Error [unbounded_send]: {}", e);
                    }
                }
            }

            std::mem::drop(peers);

            thread::sleep(time::Duration::from_millis(47000));
        }
    });

    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}


async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    let guard = peer_map.lock().unwrap().insert(addr, tx);
    std::mem::drop(guard);

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        let msg_in = msg.to_text().unwrap();
        println!("Received a message from {}", addr);
        let peers = peer_map.lock().unwrap();

        //Add a comment if it's not blank
        let json_result = serde_json::from_str(msg_in);
        match json_result {
            Ok(comment) => {
                let mut comment: mkultra::Comment = comment;
                let msg_id: u64 = comment.id;
                let chat_id: String = comment.chat_id.clone();

                //Message types below -1 are administrative and should not contain a payload
                if comment.kind > -2  {

                    //Create an event from this comment and attempt to verify it
                    let event = Event {
                        id: "".to_string(),
                        pub_key: comment.pubkey.clone(),
                        created_at: comment.created_at,
                        kind: comment.kind,
                        tags: vec![],
                        content: comment.content.clone(),
                        sig: comment.sig.clone(),
                    };

                    match event.verify() {
                        Ok(_) => {
                            //Add the comment to the database
                            if !comment.content.trim().is_empty() && !comment.content.contains("<script>")  {
                                comment.id = 0;
                                match mkultra::add_comment_to_db(comment) {
                                    Ok(_) => {
                                        println!("Comment added.\n");
                                    }
                                    Err(e) => {
                                        eprintln!("Error adding comment: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error verifying event signature: [{:#?}]", e);
                        }
                    }
                }

                //Get the comments
                let comments = mkultra::get_comments_by_chat_id(&chat_id, msg_id).unwrap();
                let json_comments = serde_json::to_string(&comments).unwrap();

                // We want to broadcast the message to everyone except ourselves.
                let broadcast_recipients =
                    peers.iter().filter(|(peer_addr, _)| peer_addr == peer_addr).map(|(_, ws_sink)| ws_sink);

                for recp in broadcast_recipients {
                    let result = recp.unbounded_send( Message::Text(json_comments.clone().into()) );
                    match result {
                        Ok(_) => {
                            println!("  Message sent.");
                        }
                        Err(e) => {
                            eprintln!("  Error [unbounded_send]: {}", e);
                        }
                    }
                }
            }
            Err (e) => {
                eprintln!("  Error unwrapping json: {}", e);
            }
        }


        std::mem::drop(peers);

        future::ok(())
    });


    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{} disconnected", &addr);
    let guard = peer_map.lock().unwrap().remove(&addr);
    std::mem::drop(guard);
}


/// Event is the struct used to represent a Nostr event
#[derive(Serialize, Deserialize, Debug)]
pub struct Event {
    /// 32-bytes sha256 of the serialized event data
    pub id: String,
    /// 32-bytes hex-encoded public key of the event creator
    #[serde(rename = "pubkey")]
    pub pub_key: String,
    /// unix timestamp in seconds
    pub created_at: u64,
    /// integer
    /// 0: NostrEvent
    pub kind: i32,
    /// Tags
    pub tags: Vec<Vec<String>>,
    /// arbitrary string
    pub content: String,
    /// 64-bytes signature of the sha256 hash of the serialized event data, which is the same as the "id" field
    pub sig: String,
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum EventError {
    #[error("Secp256k1 Error: {}", _0)]
    Secp256k1Error(secp256k1::Error),
}

impl From<secp256k1::Error> for EventError {
    fn from(err: secp256k1::Error) -> Self {
        Self::Secp256k1Error(err)
    }
}

impl Event {
    pub fn get_content(&self) -> String {
        json!([
            0,
            self.pub_key,
            self.created_at,
            self.kind,
            self.tags,
            self.content
        ])
            .to_string()
    }

    pub fn get_content_id(&self) -> String {
        sha256::digest(self.get_content())
    }

    pub fn verify(&self) -> Result<(), EventError> {
        let message = secp256k1::Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(
            self.get_content().as_bytes(),
        );

        SECP256K1.verify_schnorr(
            &Signature::from_str(&self.sig)?,
            &message,
            &XOnlyPublicKey::from_str(&self.pub_key)?,
        )?;
        Ok(())
    }
}

impl fmt::Display for Event {
    /// Return the serialized event
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(&self).unwrap())
    }
}
